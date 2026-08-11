//! Regression coverage for per-attempt deadlines across redirect hops.

mod support;

use std::time::Duration;

use serde_json::json;
use support::http_server::{ScriptedResponse, TestServer};
use zai_rs::{
    client::{ApiFamily, HttpTransportConfig, ZaiClient, error::codes},
    file::{FileListPurpose, FileListRequest},
};

const KEY: &str = "test.12345678901234567890";
const ATTEMPT_TIMEOUT: Duration = Duration::from_secs(10);
const HOP_DELAY: Duration = Duration::from_secs(4);

fn redirect(location: &str) -> ScriptedResponse {
    let mut response = ScriptedResponse::empty(307).with_delay(HOP_DELAY);
    response.headers.push(("location".into(), location.into()));
    response
}

fn file_list_response() -> serde_json::Value {
    json!({
        "object": "list",
        "data": [],
        "has_more": false
    })
}

async fn wait_for_requests(server: &TestServer, expected: usize) {
    for _ in 0..10_000 {
        if server.requests().len() >= expected {
            return;
        }
        // Keep a ready future in the paused runtime until loopback I/O has
        // reached the server, instead of letting Tokio auto-advance deadlines.
        tokio::task::yield_now().await;
    }
    panic!(
        "server captured {} requests, expected at least {expected}",
        server.requests().len()
    );
}

async fn wait_for_retry(server: &TestServer) {
    for _ in 0..20 {
        if server.requests().len() >= 4 {
            return;
        }
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_millis(50)).await;
    }
    panic!(
        "server captured {} requests; redirect hops did not time out as one attempt",
        server.requests().len()
    );
}

#[tokio::test(start_paused = true)]
async fn redirect_hops_share_one_attempt_deadline() {
    let server = TestServer::start(vec![
        redirect("/api/paas/v4/redirect-1"),
        redirect("/api/paas/v4/redirect-2"),
        ScriptedResponse::json(200, file_list_response()).with_delay(HOP_DELAY),
        // The first attempt reaches its cumulative deadline while waiting for
        // the preceding response. A real retry gets a fresh deadline, which is
        // long enough for this deliberately slow success response.
        ScriptedResponse::json(200, file_list_response()).with_delay(Duration::from_secs(5)),
    ])
    .await;
    let transport = HttpTransportConfig::default()
        .with_request_timeout(ATTEMPT_TIMEOUT)
        .unwrap()
        .with_max_attempts(2)
        .unwrap();
    let base_url = format!("{}/api/paas/v4", server.base_url);
    let client = ZaiClient::builder(KEY)
        .allow_insecure_transport(true)
        .endpoint(ApiFamily::PaasV4, base_url)
        .transport(transport)
        .build()
        .unwrap();

    let request = tokio::spawn(async move {
        FileListRequest::new(FileListPurpose::Batch)
            .send_via(&client)
            .await
    });

    wait_for_requests(&server, 1).await;
    tokio::time::advance(HOP_DELAY).await;
    wait_for_requests(&server, 2).await;
    tokio::time::advance(HOP_DELAY).await;
    wait_for_requests(&server, 3).await;

    // Only two seconds remain in the attempt. The third four-second response
    // must therefore time out cumulatively; bounded jitter is at most 200 ms.
    tokio::time::advance(Duration::from_secs(2)).await;
    wait_for_retry(&server).await;

    tokio::time::advance(Duration::from_secs(5)).await;
    let response = request
        .await
        .unwrap()
        .expect("the retry should receive a fresh attempt deadline");

    assert_eq!(response.has_more, Some(false));
    let requests = server.requests();
    assert_eq!(
        requests.len(),
        4,
        "the cumulative redirect delay must time out the first attempt"
    );
    assert_eq!(requests[0].path, "/api/paas/v4/files");
    assert_eq!(requests[1].path, "/api/paas/v4/redirect-1");
    assert_eq!(requests[2].path, "/api/paas/v4/redirect-2");
    server.shutdown().await;
}

#[tokio::test]
async fn redirect_without_location_is_terminal_and_never_replayed() {
    let server = TestServer::start(vec![
        ScriptedResponse::empty(307),
        ScriptedResponse::json(200, file_list_response()),
    ])
    .await;
    let base_url = format!("{}/api/paas/v4", server.base_url);
    let client = ZaiClient::builder(KEY)
        .allow_insecure_transport(true)
        .endpoint(ApiFamily::PaasV4, base_url)
        .build()
        .unwrap();

    FileListRequest::new(FileListPurpose::Batch)
        .send_via(&client)
        .await
        .unwrap_err();

    assert_eq!(
        server.requests().len(),
        1,
        "a missing Location must not resolve back to the current URL"
    );
    server.shutdown().await;
}

#[tokio::test]
async fn rejected_redirect_preserves_policy_error_without_echoing_location() {
    let secret = "private-location-token";
    let mut response = ScriptedResponse::empty(307);
    response
        .headers
        .push(("location".into(), format!("https://other.example/{secret}")));
    response
        .headers
        .push(("x-request-id".into(), "redirect-current".into()));
    response.headers.push(("retry-after".into(), "7".into()));
    let server = TestServer::start(vec![response]).await;
    let client = ZaiClient::builder(KEY)
        .allow_insecure_transport(true)
        .endpoint(
            ApiFamily::PaasV4,
            format!("{}/api/paas/v4", server.base_url),
        )
        .build()
        .unwrap();

    let error = FileListRequest::new(FileListPurpose::Batch)
        .send_via(&client)
        .await
        .expect_err("a cross-origin redirect must be rejected");

    assert_eq!(error.code(), Some(codes::SDK_VALIDATION));
    assert!(error.message().contains("cross-origin redirect refused"));
    for rendered in [error.to_string(), format!("{error:?}"), error.compact()] {
        assert!(!rendered.contains(secret));
        assert!(!rendered.contains("other.example"));
    }
    let metadata = error.request_metadata().unwrap();
    assert_eq!(metadata.attempts(), 1);
    assert_eq!(metadata.request_id(), Some("redirect-current"));
    assert_eq!(metadata.retry_after(), Some(Duration::from_secs(7)));
    assert_eq!(server.requests().len(), 1);
    server.shutdown().await;
}

#[tokio::test]
async fn rejected_redirect_metadata_does_not_reuse_the_previous_retry_response() {
    let mut retryable = ScriptedResponse::empty(503);
    retryable
        .headers
        .push(("x-request-id".into(), "stale-first-attempt".into()));
    retryable.headers.push(("retry-after".into(), "0".into()));

    let mut rejected = ScriptedResponse::empty(307);
    rejected
        .headers
        .push(("location".into(), "https://other.example/rejected".into()));
    rejected
        .headers
        .push(("x-request-id".into(), "redirect-second-attempt".into()));
    rejected.headers.push(("retry-after".into(), "9".into()));

    let server = TestServer::start(vec![retryable, rejected]).await;
    let transport = HttpTransportConfig::default().with_max_attempts(2).unwrap();
    let client = ZaiClient::builder(KEY)
        .allow_insecure_transport(true)
        .endpoint(
            ApiFamily::PaasV4,
            format!("{}/api/paas/v4", server.base_url),
        )
        .transport(transport)
        .build()
        .unwrap();

    let error = FileListRequest::new(FileListPurpose::Batch)
        .send_via(&client)
        .await
        .expect_err("the cross-origin retry response must be rejected");

    assert_eq!(error.code(), Some(codes::SDK_VALIDATION));
    let metadata = error.request_metadata().unwrap();
    assert_eq!(metadata.attempts(), 2);
    assert_eq!(metadata.request_id(), Some("redirect-second-attempt"));
    assert_eq!(metadata.retry_after(), Some(Duration::from_secs(9)));
    assert_ne!(metadata.request_id(), Some("stale-first-attempt"));
    assert_eq!(server.requests().len(), 2);
    server.shutdown().await;
}

#[tokio::test(start_paused = true)]
async fn followed_redirect_metadata_survives_a_target_timeout() {
    let mut followed = ScriptedResponse::empty(307);
    followed
        .headers
        .push(("location".into(), "/api/paas/v4/slow-target".into()));
    followed
        .headers
        .push(("x-request-id".into(), "redirect-before-timeout".into()));
    followed.headers.push(("retry-after".into(), "6".into()));
    let server = TestServer::start(vec![
        followed,
        ScriptedResponse::json(200, file_list_response()).with_delay(Duration::from_secs(10)),
    ])
    .await;
    let transport = HttpTransportConfig::default()
        .with_request_timeout(Duration::from_secs(1))
        .unwrap()
        .with_max_attempts(1)
        .unwrap();
    let client = ZaiClient::builder(KEY)
        .allow_insecure_transport(true)
        .endpoint(
            ApiFamily::PaasV4,
            format!("{}/api/paas/v4", server.base_url),
        )
        .transport(transport)
        .build()
        .unwrap();

    let request = tokio::spawn(async move {
        FileListRequest::new(FileListPurpose::Batch)
            .send_via(&client)
            .await
    });
    wait_for_requests(&server, 2).await;
    tokio::time::advance(Duration::from_secs(1)).await;
    let error = request
        .await
        .unwrap()
        .expect_err("the redirected target should exceed the shared attempt deadline");

    let metadata = error.request_metadata().unwrap();
    assert_eq!(metadata.attempts(), 1);
    assert_eq!(metadata.request_id(), Some("redirect-before-timeout"));
    assert_eq!(metadata.retry_after(), Some(Duration::from_secs(6)));
    assert_eq!(server.requests().len(), 2);
    server.shutdown().await;
}
