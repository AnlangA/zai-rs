//! Unknown business codes must not hide actionable HTTP recovery semantics.

mod support;

use serde_json::{Value, json};
use support::http_server::{ScriptedResponse, TestServer};
use zai_rs::client::{ApiFamily, ErrorCategory, ZaiClient};
use zai_rs::model::{ChatCompletion, GLM5_2, TextMessage};

const TEST_KEY: &str = "test.12345678901234567890";

fn client_for(server: &TestServer) -> ZaiClient {
    ZaiClient::builder(TEST_KEY)
        .allow_insecure_transport(true)
        .endpoint(
            ApiFamily::PaasV4,
            format!("{}/api/paas/v4", server.base_url),
        )
        .build()
        .unwrap()
}

#[tokio::test]
async fn unknown_number_text_and_large_codes_fall_back_to_http_semantics() {
    let code_cases: [(Value, &str); 3] = [
        (json!(7777), "7777"),
        (json!("UPSTREAM_BUSY"), r#""UPSTREAM_BUSY""#),
        (json!(70_000), "70000"),
    ];
    let status_cases = [
        (401, ErrorCategory::Auth, false),
        (429, ErrorCategory::RateLimit, true),
        (503, ErrorCategory::Server, true),
    ];

    for (status, category, retryable) in status_cases {
        for (wire_code, diagnostic) in &code_cases {
            let server = TestServer::start(vec![ScriptedResponse::json(
                status,
                json!({
                    "error": {
                        "code": wire_code,
                        "message": "upstream rejected request"
                    }
                }),
            )])
            .await;
            let client = client_for(&server);

            let error = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hello"))
                .send_via(&client)
                .await
                .expect_err("the business error envelope must not decode as success");

            assert_eq!(error.code(), Some(status));
            assert_eq!(error.raw_business_code(), Some(*diagnostic));
            assert_eq!(error.category(), category);
            assert_eq!(error.is_retryable(), retryable);
            assert!(
                !error.to_string().contains(diagnostic),
                "Display must not emit the diagnostic business code"
            );
            assert!(
                !format!("{error:?}").contains(diagnostic),
                "Debug must not emit the diagnostic business code"
            );
            server.shutdown().await;
        }
    }
}
