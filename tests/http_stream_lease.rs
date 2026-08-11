//! End-to-end contracts for reclaiming abandoned HTTP response streams.

mod support;

use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use serde_json::json;
use support::http_server::{ScriptedResponse, TestServer};
use zai_rs::client::{
    ApiFamily, HttpConcurrencyConfig, HttpTransportConfig, RequestOptions, TimeoutPhase, ZaiClient,
    ZaiError,
};
use zai_rs::file::{FileContentRequest, FileListPurpose, FileListRequest};
use zai_rs::model::{ChatCompletion, GLM5_2, TextMessage};

const TEST_KEY: &str = "test.12345678901234567890";
const TEST_TIMEOUT: Duration = Duration::from_secs(4);
const PREDISPATCH_WINDOW: Duration = Duration::from_millis(100);

fn client_for(server: &TestServer, stream_consumer_base: Duration) -> ZaiClient {
    let transport = HttpTransportConfig::default()
        .with_request_timeout(Duration::from_secs(10))
        .unwrap()
        .with_max_attempts(1)
        .unwrap();
    let concurrency = HttpConcurrencyConfig::default()
        .with_max_in_flight(1)
        .unwrap()
        .with_queue_timeout(TEST_TIMEOUT)
        .unwrap()
        .with_stream_consumer_timeout(stream_consumer_base)
        .unwrap();
    ZaiClient::builder(TEST_KEY)
        .allow_insecure_transport(true)
        .endpoint(
            ApiFamily::PaasV4,
            format!("{}/api/paas/v4", server.base_url),
        )
        .transport(transport)
        .concurrency(concurrency)
        .build()
        .unwrap()
}

fn empty_file_list() -> serde_json::Value {
    json!({"object": "list", "data": [], "has_more": false})
}

fn held_stream(content_type: &str, body: &'static [u8]) -> ScriptedResponse {
    ScriptedResponse::chunked(200, content_type, [Bytes::from_static(body)])
        .with_chunk_gate(Arc::new(tokio::sync::Semaphore::new(0)))
}

async fn wait_for_requests(server: &TestServer, expected: usize) {
    tokio::time::timeout(TEST_TIMEOUT, async {
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

fn assert_timeout(error: &ZaiError, phase: TimeoutPhase) {
    assert_eq!(
        error.code(),
        Some(zai_rs::client::error::codes::SDK_TIMEOUT)
    );
    let metadata = error
        .request_metadata()
        .expect("stream timeouts must retain request metadata");
    assert_eq!(metadata.attempts(), 1);
    assert_eq!(metadata.timeout_phase(), Some(phase));
}

async fn list_files(client: &ZaiClient) -> zai_rs::ZaiResult<zai_rs::file::FileListResponse> {
    FileListRequest::new(FileListPurpose::Batch)
        .send_via(client)
        .await
}

fn fail_fast(client: &ZaiClient) -> ZaiClient {
    client.clone().with_request_options(
        RequestOptions::default()
            .with_queue_timeout(Duration::ZERO)
            .unwrap(),
    )
}

#[tokio::test]
async fn unpolled_sse_consumer_lease_reclaims_the_body_and_only_permit() {
    let server = TestServer::start(vec![
        held_stream("text/event-stream", b"data: [DONE]\n\n"),
        ScriptedResponse::json(200, empty_file_list()),
    ])
    .await;
    // The request-level value lowers the client-wide base. The 1 ms idle
    // timeout plus its one-second floor then controls the effective lease.
    let client = client_for(&server, Duration::from_secs(10));
    let scoped = client.clone().with_request_options(
        RequestOptions::default()
            .with_sse_idle_timeout(Duration::from_millis(1))
            .unwrap()
            .with_stream_consumer_timeout(Duration::from_millis(50))
            .unwrap(),
    );

    let mut stream = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hold lease"))
        .enable_stream()
        .stream_via(&scoped)
        .await
        .unwrap();
    wait_for_requests(&server, 1).await;

    // Keep the first stream alive and completely unpolled while the second
    // operation is driven far enough to wait for the only admission permit.
    let second = list_files(&client);
    tokio::pin!(second);
    tokio::select! {
        result = &mut second => panic!("second request completed before lease expiry: {result:?}"),
        () = tokio::time::sleep(PREDISPATCH_WINDOW) => {},
    }
    assert_eq!(
        server.requests().len(),
        1,
        "queued request reached the server before stream reclamation"
    );

    let response = tokio::time::timeout(TEST_TIMEOUT, &mut second)
        .await
        .expect("consumer lease did not release the admission permit")
        .expect("buffered request failed after consumer-lease reclamation");
    assert_eq!(response.has_more, Some(false));
    assert_eq!(server.requests().len(), 2);

    let error = tokio::time::timeout(Duration::from_millis(250), stream.next())
        .await
        .expect("reclaimed SSE stream did not wake its consumer")
        .expect("reclaimed SSE stream ended without its timeout")
        .expect_err("reclaimed SSE stream yielded data instead of its timeout");
    assert_timeout(&error, TimeoutPhase::StreamConsumer);
    assert!(
        tokio::time::timeout(Duration::from_millis(250), stream.next())
            .await
            .expect("SSE stream did not terminate after its timeout")
            .is_none(),
        "timeout must be yielded once"
    );
    server.shutdown().await;
}

#[tokio::test]
async fn unpolled_file_overall_deadline_reclaims_the_body_and_only_permit() {
    let server = TestServer::start(vec![
        held_stream("application/octet-stream", b"unread file bytes"),
        ScriptedResponse::json(200, empty_file_list()),
    ])
    .await;
    let client = client_for(&server, Duration::from_secs(10));
    let scoped = client.clone().with_request_options(
        RequestOptions::default()
            .with_overall_timeout(Duration::from_secs(1))
            .unwrap()
            .with_max_attempts(1)
            .unwrap(),
    );

    let mut stream = FileContentRequest::new("held")
        .stream_via(&scoped)
        .await
        .unwrap();
    wait_for_requests(&server, 1).await;

    let second = list_files(&client);
    tokio::pin!(second);
    tokio::select! {
        result = &mut second => panic!("second request completed before overall expiry: {result:?}"),
        () = tokio::time::sleep(PREDISPATCH_WINDOW) => {},
    }
    assert_eq!(
        server.requests().len(),
        1,
        "queued request reached the server before stream reclamation"
    );

    let response = tokio::time::timeout(TEST_TIMEOUT, &mut second)
        .await
        .expect("overall deadline did not release the admission permit")
        .expect("buffered request failed after overall-deadline reclamation");
    assert_eq!(response.has_more, Some(false));
    assert_eq!(server.requests().len(), 2);

    let error = tokio::time::timeout(Duration::from_millis(250), stream.next())
        .await
        .expect("reclaimed file stream did not wake its consumer")
        .expect("reclaimed file stream ended without its timeout")
        .expect_err("reclaimed file stream yielded data instead of its timeout");
    assert_timeout(&error, TimeoutPhase::Overall);
    assert!(
        tokio::time::timeout(Duration::from_millis(250), stream.next())
            .await
            .expect("file stream did not terminate after its timeout")
            .is_none(),
        "timeout must be yielded once"
    );
    server.shutdown().await;
}

#[tokio::test]
async fn caller_drop_still_releases_an_unpolled_stream_immediately() {
    let server = TestServer::start(vec![
        held_stream("text/event-stream", b"data: [DONE]\n\n"),
        ScriptedResponse::json(200, empty_file_list()),
        held_stream("application/octet-stream", b"unread file bytes"),
        ScriptedResponse::json(200, empty_file_list()),
    ])
    .await;
    let client = client_for(&server, Duration::from_secs(10));

    let sse = ChatCompletion::new(GLM5_2 {}, TextMessage::user("drop SSE"))
        .enable_stream()
        .stream_via(&client)
        .await
        .unwrap();
    wait_for_requests(&server, 1).await;
    drop(sse);
    tokio::time::timeout(TEST_TIMEOUT, list_files(&fail_fast(&client)))
        .await
        .expect("buffered request stalled after dropping SSE")
        .expect("dropping SSE did not release the permit synchronously");
    assert_eq!(server.requests().len(), 2);

    let file = FileContentRequest::new("drop-file")
        .stream_via(&client)
        .await
        .unwrap();
    wait_for_requests(&server, 3).await;
    drop(file);
    tokio::time::timeout(TEST_TIMEOUT, list_files(&fail_fast(&client)))
        .await
        .expect("buffered request stalled after dropping file stream")
        .expect("dropping file stream did not release the permit synchronously");
    assert_eq!(server.requests().len(), 4);
    server.shutdown().await;
}
