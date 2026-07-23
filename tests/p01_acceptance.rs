//! Integration tests for business-error envelope detection.
//!
//! A 2xx body that carries a business error envelope
//! (`{code:500,...}` or `{"error":{...}}`) must return `Err`, not decode into
//! an all-optional success type. These tests drive the real SDK chat path
//! through `ZaiClient` against the shared scripted mock server, which returns
//! HTTP 200 with an error-shaped body.

mod support;

use serde_json::{Value, json};
use support::http_server::{ScriptedResponse, TestServer};

use zai_rs::client::{ApiFamily, ZaiClient};
use zai_rs::model::{ChatCompletion, GLM5_2, TextMessage};

const TEST_KEY: &str = "test.12345678901234567890";

/// Build a `ZaiClient` whose PaasV4 endpoint points at the mock base.
fn client_for_mock(base: &str) -> ZaiClient {
    ZaiClient::builder(TEST_KEY)
        .allow_insecure_transport(true)
        .endpoint(ApiFamily::PaasV4, base)
        .build()
        .unwrap()
}

/// Start a mock server that responds with HTTP 200 and the given JSON body,
/// returning the server and the base URL the SDK should point at.
async fn mock_200_with_body(body: Value) -> (TestServer, String) {
    let server = TestServer::start(vec![ScriptedResponse::json(200, body)]).await;
    let base = format!("{}/api/paas/v4", server.base_url);
    (server, base)
}

#[tokio::test]
async fn two_xx_with_code_500_business_error_returns_err() {
    let (server, base) =
        mock_200_with_body(json!({"code": 500, "message": "internal error"})).await;
    let client = client_for_mock(&base);
    let error = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hello"))
        .send_via(&client)
        .await
        .unwrap_err();
    assert_eq!(error.code(), Some(500));
    server.shutdown().await;
}

#[tokio::test]
async fn two_xx_with_nested_error_envelope_returns_err() {
    let (server, base) = mock_200_with_body(json!({
        "error": {"code": 1302, "message": "rate limited"}
    }))
    .await;
    let client = client_for_mock(&base);
    let error = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hello"))
        .send_via(&client)
        .await
        .unwrap_err();
    assert_eq!(error.code(), Some(1302));
    server.shutdown().await;
}
