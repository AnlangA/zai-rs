//! End-to-end contracts for shared HTTP admission control.

mod support;

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

use bytes::Bytes;
use serde_json::json;
use support::http_server::{ScriptedResponse, TestServer};
use zai_rs::client::{
    ApiFamily, HttpConcurrencyConfig, HttpTransportConfig, RequestOptions, TimeoutPhase, ZaiClient,
};
use zai_rs::file::{FileListPurpose, FileListRequest};

const TEST_KEY: &str = "test.12345678901234567890";
const TEST_TIMEOUT: Duration = Duration::from_secs(5);

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

fn client_for(
    server: &TestServer,
    transport: HttpTransportConfig,
    concurrency: HttpConcurrencyConfig,
) -> ZaiClient {
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

fn one_attempt_transport(timeout: Duration) -> HttpTransportConfig {
    HttpTransportConfig::default()
        .with_request_timeout(timeout)
        .unwrap()
        .with_max_attempts(1)
        .unwrap()
}

fn fail_fast(client: &ZaiClient) -> ZaiClient {
    client.clone().with_request_options(
        RequestOptions::default()
            .with_queue_timeout(Duration::ZERO)
            .unwrap(),
    )
}

async fn list_files(client: &ZaiClient) -> zai_rs::ZaiResult<zai_rs::file::FileListResponse> {
    FileListRequest::new(FileListPurpose::Batch)
        .send_via(client)
        .await
}

async fn wait_for_requests(server: &TestServer, count: usize) {
    tokio::time::timeout(TEST_TIMEOUT, async {
        while server.requests().len() < count {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("server did not receive {count} requests"));
}

async fn wait_for_counter(counter: &AtomicUsize, count: usize) {
    tokio::time::timeout(TEST_TIMEOUT, async {
        while counter.load(Ordering::SeqCst) < count {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("script did not emit {count} chunks"));
}

fn assert_queue_timeout(error: &zai_rs::ZaiError) {
    assert_eq!(
        error.code(),
        Some(zai_rs::client::error::codes::SDK_TIMEOUT)
    );
    assert!(error.message().contains("concurrency queue"));
    assert!(error.is_retryable());
    let metadata = error
        .request_metadata()
        .expect("queue timeout must carry request metadata");
    assert_eq!(metadata.attempts(), 0);
    assert_eq!(metadata.timeout_phase(), Some(TimeoutPhase::Queue));
    assert_eq!(metadata.request_id(), None);
    assert_eq!(metadata.retry_after(), None);
}

#[tokio::test]
async fn zero_queue_timeout_succeeds_when_idle_and_fails_fast_when_saturated() {
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let server = TestServer::start(vec![
        ScriptedResponse::json(200, empty_file_list()),
        gated_file_list(Arc::clone(&gate)),
    ])
    .await;
    let concurrency = HttpConcurrencyConfig::default()
        .with_max_in_flight(1)
        .unwrap()
        .with_queue_timeout(Duration::from_secs(2))
        .unwrap();
    let client = client_for(
        &server,
        one_attempt_transport(Duration::from_secs(2)),
        concurrency,
    );
    let fail_fast_client = fail_fast(&client);

    let idle = list_files(&fail_fast_client).await.unwrap();
    assert_eq!(idle.has_more, Some(false));

    let active_client = client.clone();
    let active = tokio::spawn(async move { list_files(&active_client).await });
    wait_for_requests(&server, 2).await;

    let error = tokio::time::timeout(Duration::from_secs(1), list_files(&fail_fast_client))
        .await
        .expect("zero queue timeout did not fail fast")
        .unwrap_err();
    assert_queue_timeout(&error);
    assert_eq!(server.requests().len(), 2, "timed-out request was sent");

    gate.add_permits(1);
    active
        .await
        .expect("active request task panicked")
        .expect("active request failed");
    server.shutdown().await;
}

#[tokio::test]
async fn queue_wait_does_not_consume_attempt_or_overall_deadlines() {
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let server = TestServer::start(vec![
        gated_file_list(Arc::clone(&gate)),
        ScriptedResponse::json(200, empty_file_list()).with_delay(Duration::from_millis(150)),
    ])
    .await;
    let concurrency = HttpConcurrencyConfig::default()
        .with_max_in_flight(1)
        .unwrap()
        .with_queue_timeout(Duration::from_secs(2))
        .unwrap();
    let client = client_for(
        &server,
        one_attempt_transport(Duration::from_secs(3)),
        concurrency,
    );

    let active_client = client.clone();
    let active = tokio::spawn(async move { list_files(&active_client).await });
    wait_for_requests(&server, 1).await;

    let queued_client = client.clone().with_request_options(
        RequestOptions::default()
            .with_queue_timeout(Duration::from_secs(2))
            .unwrap()
            .with_attempt_timeout(Duration::from_millis(500))
            .unwrap()
            .with_overall_timeout(Duration::from_millis(500))
            .unwrap(),
    );
    let queued = tokio::spawn(async move { list_files(&queued_client).await });

    // Wait longer than both scoped HTTP deadlines. The request must remain in
    // admission rather than consuming either deadline while no slot exists.
    tokio::time::sleep(Duration::from_millis(700)).await;
    assert_eq!(server.requests().len(), 1, "queued request was dispatched");

    gate.add_permits(1);
    active
        .await
        .expect("active request task panicked")
        .expect("active request failed");
    let response = tokio::time::timeout(TEST_TIMEOUT, queued)
        .await
        .expect("queued request never completed")
        .expect("queued request task panicked")
        .expect("queue wait consumed the HTTP deadline");
    assert_eq!(response.has_more, Some(false));
    assert_eq!(server.requests().len(), 2);
    server.shutdown().await;
}

#[tokio::test]
async fn clones_share_one_limiter_but_independent_clients_do_not() {
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let server = TestServer::start(vec![
        gated_file_list(Arc::clone(&gate)),
        ScriptedResponse::json(200, empty_file_list()),
    ])
    .await;
    let concurrency = HttpConcurrencyConfig::default()
        .with_max_in_flight(1)
        .unwrap()
        .with_queue_timeout(Duration::from_secs(2))
        .unwrap();
    let transport = one_attempt_transport(Duration::from_secs(2));
    let first_client = client_for(&server, transport.clone(), concurrency.clone());
    let independent_client = client_for(&server, transport, concurrency);

    let active_client = first_client.clone();
    let active = tokio::spawn(async move { list_files(&active_client).await });
    wait_for_requests(&server, 1).await;

    let clone_error = list_files(&fail_fast(&first_client)).await.unwrap_err();
    assert_queue_timeout(&clone_error);
    assert_eq!(server.requests().len(), 1, "clone bypassed shared limiter");

    let independent = tokio::time::timeout(TEST_TIMEOUT, list_files(&independent_client))
        .await
        .expect("independent client was blocked by another client's limiter")
        .expect("independent client request failed");
    assert_eq!(independent.has_more, Some(false));
    assert_eq!(server.requests().len(), 2);

    gate.add_permits(1);
    active
        .await
        .expect("active request task panicked")
        .expect("active request failed");
    server.shutdown().await;
}

#[tokio::test]
async fn retry_backoff_keeps_the_logical_request_permit() {
    let emitted = Arc::new(AtomicUsize::new(0));
    let mut retryable = ScriptedResponse::chunked(
        503,
        "application/json",
        [Bytes::from_static(br#"{"message":"busy"}"#)],
    )
    .with_chunk_counter(Arc::clone(&emitted));
    retryable.headers.push(("retry-after".into(), "1".into()));
    let server = TestServer::start(vec![
        retryable,
        ScriptedResponse::json(200, empty_file_list()),
        ScriptedResponse::json(200, empty_file_list()),
    ])
    .await;
    let transport = HttpTransportConfig::default()
        .with_request_timeout(Duration::from_secs(3))
        .unwrap()
        .with_max_attempts(2)
        .unwrap();
    let concurrency = HttpConcurrencyConfig::default()
        .with_max_in_flight(1)
        .unwrap()
        .with_queue_timeout(Duration::from_secs(3))
        .unwrap();
    let client = client_for(&server, transport, concurrency);

    let retrying_client = client.clone();
    let retrying = tokio::spawn(async move { list_files(&retrying_client).await });
    wait_for_counter(&emitted, 1).await;
    // The Retry-After delay is one second. Give the client ample time to enter
    // backoff while staying well before the second attempt.
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert_eq!(server.requests().len(), 1);

    let error = list_files(&fail_fast(&client)).await.unwrap_err();
    assert_queue_timeout(&error);
    assert_eq!(
        server.requests().len(),
        1,
        "another request was admitted between retry attempts"
    );

    retrying
        .await
        .expect("retrying task panicked")
        .expect("retrying request failed");
    assert_eq!(server.requests().len(), 2);
    list_files(&client).await.unwrap();
    assert_eq!(server.requests().len(), 3);
    server.shutdown().await;
}
