//! End-to-end contracts for shared HTTP logical-request admission.

mod support;

use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use serde_json::json;
use support::http_server::{ScriptedResponse, TestServer};
use zai_rs::client::{
    ApiFamily, HttpConcurrencyConfig, RequestOptions, TimeoutPhase, ZaiClient, ZaiError,
};
use zai_rs::file::{FileContentRequest, FileListPurpose, FileListRequest};
use zai_rs::model::{ChatCompletion, GLM5_2, TextMessage};

const TEST_KEY: &str = "test.12345678901234567890";

fn concurrency(queue_timeout: Duration) -> HttpConcurrencyConfig {
    HttpConcurrencyConfig::default()
        .with_max_in_flight(1)
        .unwrap()
        .with_queue_timeout(queue_timeout)
        .unwrap()
}

fn client_for(server: &TestServer, queue_timeout: Duration) -> ZaiClient {
    ZaiClient::builder(TEST_KEY)
        .allow_insecure_transport(true)
        .endpoint(
            ApiFamily::PaasV4,
            format!("{}/api/paas/v4", server.base_url),
        )
        .concurrency(concurrency(queue_timeout))
        .build()
        .unwrap()
}

fn empty_file_list() -> serde_json::Value {
    json!({"object": "list", "data": [], "has_more": false})
}

fn gated_file_list(gate: Arc<tokio::sync::Semaphore>) -> ScriptedResponse {
    ScriptedResponse::chunked(
        200,
        "application/json",
        [Bytes::from(empty_file_list().to_string())],
    )
    .with_chunk_gate(gate)
}

async fn wait_for_requests(server: &TestServer, expected: usize) {
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if server.requests().len() >= expected {
                return;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("scripted server did not receive the expected request count");
}

fn assert_queue_timeout(error: &ZaiError) {
    assert_eq!(
        error.code(),
        Some(zai_rs::client::error::codes::SDK_TIMEOUT)
    );
    let metadata = error
        .request_metadata()
        .expect("queue timeouts must carry request metadata");
    assert_eq!(metadata.attempts(), 0);
    assert_eq!(metadata.timeout_phase(), Some(TimeoutPhase::Queue));
    assert!(metadata.request_id().is_none());
}

async fn spawn_held_buffered_request(
    client: &ZaiClient,
    server: &TestServer,
) -> tokio::task::JoinHandle<zai_rs::ZaiResult<zai_rs::file::FileListResponse>> {
    let held_client = client.clone();
    let held = tokio::spawn(async move {
        FileListRequest::new(FileListPurpose::Batch)
            .send_via(&held_client)
            .await
    });
    wait_for_requests(server, 1).await;
    held
}

#[tokio::test]
async fn buffered_queue_timeout_is_predispatch_and_scoped_timeout_cannot_raise_global() {
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let server = TestServer::start(vec![gated_file_list(Arc::clone(&gate))]).await;
    let client = client_for(&server, Duration::ZERO);
    let held = spawn_held_buffered_request(&client, &server).await;

    // A scoped request may ask for more time, but the client's zero-timeout
    // fail-fast policy remains the upper bound.
    let scoped = client.clone().with_request_options(
        RequestOptions::default()
            .with_queue_timeout(Duration::from_secs(60))
            .unwrap(),
    );
    let error = tokio::time::timeout(Duration::from_millis(500), async {
        FileListRequest::new(FileListPurpose::Batch)
            .send_via(&scoped)
            .await
    })
    .await
    .expect("a per-request timeout must not raise the global queue timeout")
    .unwrap_err();

    assert_queue_timeout(&error);
    assert_eq!(server.requests().len(), 1, "admission failed pre-dispatch");

    gate.add_permits(1);
    held.await.unwrap().unwrap();
    server.shutdown().await;
}

#[tokio::test]
async fn cancelling_a_queued_request_does_not_leak_the_only_permit() {
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let server = TestServer::start(vec![
        gated_file_list(Arc::clone(&gate)),
        ScriptedResponse::json(200, empty_file_list()),
    ])
    .await;
    let client = client_for(&server, Duration::from_secs(60));
    let held = spawn_held_buffered_request(&client, &server).await;

    // Poll the second operation until it is waiting for admission, then drop
    // that future. Its FIFO semaphore waiter must be removed without consuming
    // or leaking the permit released by the first operation.
    {
        let request = FileListRequest::new(FileListPurpose::Batch);
        let waiting = request.send_via(&client);
        tokio::pin!(waiting);
        tokio::select! {
            result = &mut waiting => {
                panic!("queued request completed before cancellation: {}", result.is_ok());
            },
            () = tokio::time::sleep(Duration::from_millis(25)) => {},
        }
        assert_eq!(server.requests().len(), 1);
    }

    gate.add_permits(1);
    held.await.unwrap().unwrap();

    let response = tokio::time::timeout(Duration::from_secs(2), async {
        FileListRequest::new(FileListPurpose::Batch)
            .send_via(&client)
            .await
    })
    .await
    .expect("the cancelled waiter must not leak the only permit")
    .unwrap();
    assert_eq!(response.has_more, Some(false));
    assert_eq!(server.requests().len(), 2);
    server.shutdown().await;
}

#[tokio::test]
async fn scoped_queue_timeout_can_lower_the_global_timeout() {
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let server = TestServer::start(vec![gated_file_list(Arc::clone(&gate))]).await;
    let client = client_for(&server, Duration::from_secs(60));
    let held = spawn_held_buffered_request(&client, &server).await;
    let scoped = client.clone().with_request_options(
        RequestOptions::default()
            .with_queue_timeout(Duration::ZERO)
            .unwrap(),
    );

    let error = tokio::time::timeout(Duration::from_millis(500), async {
        FileListRequest::new(FileListPurpose::Batch)
            .send_via(&scoped)
            .await
    })
    .await
    .expect("the scoped zero timeout must lower the global queue timeout")
    .unwrap_err();

    assert_queue_timeout(&error);
    assert_eq!(
        client.concurrency().queue_timeout(),
        Duration::from_secs(60)
    );
    assert_eq!(server.requests().len(), 1);

    gate.add_permits(1);
    held.await.unwrap().unwrap();
    server.shutdown().await;
}

#[tokio::test]
async fn sse_stream_holds_the_permit_until_drop_then_releases_it() {
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let server = TestServer::start(vec![
        ScriptedResponse::chunked(
            200,
            "text/event-stream",
            [Bytes::from_static(b"data: [DONE]\n\n")],
        )
        .with_chunk_gate(Arc::clone(&gate)),
        ScriptedResponse::json(200, empty_file_list()),
    ])
    .await;
    let client = client_for(&server, Duration::ZERO);

    let stream = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hold permit"))
        .enable_stream()
        .stream_via(&client)
        .await
        .unwrap();
    wait_for_requests(&server, 1).await;

    let error = FileListRequest::new(FileListPurpose::Batch)
        .send_via(&client)
        .await
        .unwrap_err();
    assert_queue_timeout(&error);
    assert_eq!(server.requests().len(), 1);

    drop(stream);
    gate.add_permits(1);
    let response = FileListRequest::new(FileListPurpose::Batch)
        .send_via(&client)
        .await
        .unwrap();
    assert_eq!(response.has_more, Some(false));
    assert_eq!(server.requests().len(), 2);
    server.shutdown().await;
}

#[tokio::test]
async fn file_stream_holds_the_permit_until_drop_then_releases_it() {
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let server = TestServer::start(vec![
        ScriptedResponse::chunked(
            200,
            "application/octet-stream",
            [Bytes::from_static(b"held file bytes")],
        )
        .with_chunk_gate(Arc::clone(&gate)),
        ScriptedResponse::json(200, empty_file_list()),
    ])
    .await;
    let client = client_for(&server, Duration::ZERO);

    let stream = FileContentRequest::new("held")
        .stream_via(&client)
        .await
        .unwrap();
    wait_for_requests(&server, 1).await;

    let error = FileListRequest::new(FileListPurpose::Batch)
        .send_via(&client)
        .await
        .unwrap_err();
    assert_queue_timeout(&error);
    assert_eq!(server.requests().len(), 1);

    drop(stream);
    gate.add_permits(1);
    let response = FileListRequest::new(FileListPurpose::Batch)
        .send_via(&client)
        .await
        .unwrap();
    assert_eq!(response.has_more, Some(false));
    assert_eq!(server.requests().len(), 2);
    server.shutdown().await;
}
