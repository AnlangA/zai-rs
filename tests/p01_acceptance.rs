//! Integration tests for business-error envelope detection.
//!
//! A 2xx body that carries a business error envelope
//! (`{code:500,...}` or `{"error":{...}}`) must return `Err`, not decode into
//! an all-optional success type. These tests drive the real SDK chat path
//! through `ZaiClient` against an inline mock that returns HTTP 200 with
//! an error-shaped body.

use std::convert::Infallible;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{Request, Response, body::Incoming, service::service_fn};
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto::Builder as ConnBuilder,
};
use serde_json::{Value, json};
use tokio::net::TcpListener;

use zai_rs::client::{ApiFamily, ZaiClient};
use zai_rs::model::{ChatCompletion, GLM5_2, TextMessage};

/// Start an inline mock that responds to one request with HTTP 200 and the
/// given JSON body, returning the base URL the SDK should point at.
async fn mock_200_with_body(body: Value) -> String {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let io = TokioIo::new(stream);
        let body = body.clone();
        let service = service_fn(move |req: Request<Incoming>| {
            let body = body.clone();
            async move {
                req.collect().await.unwrap();
                let mut resp = Response::new(Full::new(Bytes::from(body.to_string())));
                *resp.status_mut() = hyper::StatusCode::OK;
                Ok::<_, Infallible>(resp)
            }
        });
        ConnBuilder::new(TokioExecutor::new())
            .serve_connection(io, service)
            .await
            .unwrap();
    });
    format!("http://{addr}/api/paas/v4")
}

const TEST_KEY: &str = "test.12345678901234567890";

/// Build a `ZaiClient` whose PaasV4 endpoint points at the mock base.
fn client_for_mock(base: &str) -> ZaiClient {
    ZaiClient::builder(TEST_KEY)
        .allow_insecure_transport(true)
        .endpoint(ApiFamily::PaasV4, base)
        .build()
        .unwrap()
}

#[tokio::test]
async fn two_xx_with_code_500_business_error_returns_err() {
    let base = mock_200_with_body(json!({"code": 500, "message": "internal error"})).await;
    let client = client_for_mock(&base);
    let error = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hello"))
        .send_via(&client)
        .await
        .unwrap_err();
    assert_eq!(error.code(), Some(500));
}

#[tokio::test]
async fn two_xx_with_nested_error_envelope_returns_err() {
    let base = mock_200_with_body(json!({
        "error": {"code": 1302, "message": "rate limited"}
    }))
    .await;
    let client = client_for_mock(&base);
    let error = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hello"))
        .send_via(&client)
        .await
        .unwrap_err();
    assert_eq!(error.code(), Some(1302));
}
