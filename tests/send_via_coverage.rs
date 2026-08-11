//! send_via coverage: mock-server integration tests for every endpoint's
//! `send_via` path. Each test builds a ZaiClient pointing at a local mock
//! server and exercises the full request → response pipeline.

mod support;
use support::http_server::{ScriptedResponse, TestServer};

use serde_json::json;

use zai_rs::client::{ApiFamily, ZaiClient};

const KEY: &str = "test.12345678901234567890";

fn mock_client(base: &str) -> ZaiClient {
    ZaiClient::builder(KEY)
        .allow_insecure_transport(true)
        .endpoint(ApiFamily::PaasV4, base)
        .build()
        .unwrap()
}

async fn ok_server(body: serde_json::Value) -> (TestServer, ZaiClient) {
    let server = TestServer::start(vec![ScriptedResponse::json(200, body)]).await;
    let base = format!("{}/api/paas/v4", server.base_url);
    (server, mock_client(&base))
}

/// Assert the transport emitted exactly one authenticated request to the
/// endpoint under test. Content length is checked when Hyper exposes it so a
/// malformed or truncated request body cannot pass unnoticed.
fn assert_request(server: &TestServer, method: &str, path: &str) {
    let requests = server.requests();
    assert_eq!(requests.len(), 1, "expected exactly one HTTP request");

    let request = &requests[0];
    assert_eq!(request.method, method);
    assert_eq!(request.path, path);

    let authorization = request
        .authorization
        .as_deref()
        .expect("the client must attach an authorization header");
    assert!(authorization.starts_with("Bearer "));
    assert!(
        request
            .headers
            .iter()
            .any(|(name, value)| { name == "authorization" && value == authorization })
    );

    if let Some((_, value)) = request
        .headers
        .iter()
        .find(|(name, _)| name == "content-length")
    {
        assert_eq!(value.parse::<usize>().unwrap(), request.body.len());
    }
}

/// Assert that the captured request body is exactly the expected JSON value.
fn assert_json_body(server: &TestServer, expected: serde_json::Value) {
    let requests = server.requests();
    assert_eq!(requests.len(), 1, "expected exactly one HTTP request");
    let actual: serde_json::Value = serde_json::from_slice(&requests[0].body)
        .expect("captured request body must contain valid JSON");
    assert_eq!(actual, expected);
}

/// Assert that a bodyless operation did not accidentally emit JSON or form data.
fn assert_empty_body(server: &TestServer) {
    let requests = server.requests();
    assert_eq!(requests.len(), 1, "expected exactly one HTTP request");
    assert!(
        requests[0].body.is_empty(),
        "bodyless operation emitted an unexpected request body"
    );
}

/// Assert one text field in a captured multipart body.
fn assert_multipart_text_field(server: &TestServer, field_name: &str, value: &str) {
    let requests = server.requests();
    assert_eq!(requests.len(), 1, "expected exactly one HTTP request");
    let body = String::from_utf8_lossy(&requests[0].body);
    assert!(
        body.contains(&format!("name=\"{field_name}\"")),
        "multipart field `{field_name}` is missing"
    );
    assert!(
        body.lines().any(|line| line == value),
        "multipart field `{field_name}` did not contain `{value}`"
    );
}

/// Assert that the captured multipart request contains one named file part and
/// the complete payload. This exercises the path-backed streaming body rather
/// than merely checking that the endpoint returned a fixture.
fn assert_multipart_file(
    server: &TestServer,
    field_name: &str,
    file_name: &str,
    content_type: &str,
    payload: &[u8],
) {
    let requests = server.requests();
    assert_eq!(requests.len(), 1, "expected exactly one HTTP request");
    let body = &requests[0].body;
    let text = String::from_utf8_lossy(body);
    assert!(
        text.contains(&format!("name=\"{field_name}\"; filename=\"{file_name}\"")),
        "multipart file disposition is missing"
    );
    assert!(
        text.contains(&format!("Content-Type: {content_type}")),
        "multipart file content type is missing"
    );
    assert!(
        body.windows(payload.len()).any(|window| window == payload),
        "multipart file payload is missing"
    );
}

// --- Chat ---
#[tokio::test]
async fn chat_send_via() {
    let (s, c) = ok_server(json!({
        "id": "x", "model": "glm-5.2",
        "choices": [{"index": 0, "message": {"role": "assistant", "content": "hi"}, "finish_reason": "stop"}],
        "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
    })).await;
    use zai_rs::model::*;
    let resp = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hi"))
        .send_via(&c)
        .await
        .unwrap();
    assert_eq!(resp.id.as_deref(), Some("x"));
    assert_request(&s, "POST", "/api/paas/v4/chat/completions");
    s.shutdown().await;
}

#[tokio::test]
async fn chat_duplicate_business_fields_cannot_false_succeed() {
    for body in [
        r#"{"id":"private-chat","model":"glm-5.2","choices":[{"index":0,"message":{"role":"assistant","content":"private answer"},"finish_reason":"stop"}],"code":1302,"code":200}"#,
        r#"{"id":"private-chat","model":"glm-5.2","choices":[{"index":0,"message":{"role":"assistant","content":"private answer"},"finish_reason":"stop"}],"error":{"code":1302},"error":{"code":200}}"#,
    ] {
        let server =
            TestServer::start(vec![ScriptedResponse::raw(200, "application/json", body)]).await;
        let client = mock_client(&format!("{}/api/paas/v4", server.base_url));

        use zai_rs::model::*;
        let error = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hi"))
            .send_via(&client)
            .await
            .expect_err("duplicate business fields became a typed success");

        assert_eq!(
            error.code(),
            Some(zai_rs::client::error::codes::SDK_VALIDATION)
        );
        assert!(
            error
                .message()
                .contains("ambiguous JSON business-error envelope")
        );
        for rendered in [error.to_string(), format!("{error:?}"), error.compact()] {
            assert!(!rendered.contains("private-chat"));
            assert!(!rendered.contains("private answer"));
            assert!(!rendered.contains("1302"));
        }
        assert_eq!(server.requests().len(), 1);
        server.shutdown().await;
    }
}

// --- Embeddings ---
#[tokio::test]
async fn embeddings_send_via() {
    let (s, c) = ok_server(json!({
        "object": "list",
        "data": [{"index": 0, "object": "embedding", "embedding": [0.1, 0.2]}],
        "model": "embedding-2",
        "usage": {"prompt_tokens": 1, "completion_tokens": 0, "total_tokens": 1}
    }))
    .await;
    use zai_rs::model::text_embedded::*;
    let response = EmbeddingRequest::new(
        EmbeddingModel::Embedding2,
        EmbeddingInput::Single("hi".into()),
    )
    .send_via(&c)
    .await
    .unwrap();
    assert_eq!(response.data.as_deref().map(<[_]>::len), Some(1));
    assert_request(&s, "POST", "/api/paas/v4/embeddings");
    s.shutdown().await;
}

// --- Rerank ---
#[tokio::test]
async fn rerank_send_via() {
    let (s, c) = ok_server(json!({
        "created": 1,
        "id": "rerank-1",
        "results": [{"document": "d", "index": 0, "relevance_score": 0.9}],
        "usage": {"prompt_tokens": 2, "total_tokens": 2}
    }))
    .await;
    use zai_rs::model::text_rerank::*;
    let response = RerankRequest::new("q", vec!["d".into()])
        .send_via(&c)
        .await
        .unwrap();
    assert_eq!(response.id, "rerank-1");
    assert_eq!(response.results.len(), 1);
    assert_request(&s, "POST", "/api/paas/v4/rerank");
    s.shutdown().await;
}

// --- Tokenizer ---
#[tokio::test]
async fn tokenizer_send_via() {
    let (s, c) = ok_server(json!({
        "created": 1,
        "id": "tokenizer-1",
        "usage": {"prompt_tokens": 3}
    }))
    .await;
    use zai_rs::model::text_tokenizer::*;
    let response = TokenizerRequest::new(
        TokenizerModel::default(),
        vec![TokenizerMessage::User {
            content: "hi".into(),
        }],
    )
    .send_via(&c)
    .await
    .unwrap();
    assert_eq!(response.id, "tokenizer-1");
    assert_eq!(response.usage.prompt_tokens, Some(3.0));
    assert_request(&s, "POST", "/api/paas/v4/tokenizer");
    s.shutdown().await;
}

// --- Moderation ---
#[tokio::test]
async fn moderation_send_via() {
    let (s, c) = ok_server(json!({
        "id": "moderation-1",
        "created": 1,
        "result_list": [{"content_type": "text", "risk_level": "HIGH", "risk_type": []}],
        "usage": {"moderation_text": {"call_count": 1}}
    }))
    .await;
    use zai_rs::model::moderation::*;
    let response = Moderation::new_text("hi").send_via(&c).await.unwrap();
    assert_eq!(response.id.as_deref(), Some("moderation-1"));
    let results = response.result_list.unwrap();
    assert_eq!(results.len(), 1);
    assert!(matches!(results[0].risk_level, Some(RiskLevel::High)));
    assert_request(&s, "POST", "/api/paas/v4/moderations");
    s.shutdown().await;
}

// --- Image generation ---
#[tokio::test]
async fn image_gen_send_via() {
    let (s, c) = ok_server(json!({
        "created": 1,
        "data": [{"url": "https://example.com/a.png"}],
        "content_filter": [{"role": "future_provider_stage", "level": 2}]
    }))
    .await;
    use zai_rs::model::gen_image::*;
    let response = ImageGenRequest::new(zai_rs::model::gen_image::CogView4 {})
        .with_prompt("cat")
        .send_via(&c)
        .await
        .unwrap();
    assert_eq!(
        response.data().unwrap()[0].url(),
        "https://example.com/a.png"
    );
    let filter = &response.content_filter().unwrap()[0];
    assert_eq!(filter.role, None);
    assert_eq!(filter.level, Some(2));
    assert_request(&s, "POST", "/api/paas/v4/images/generations");
    s.shutdown().await;
}

// --- Video generation ---
#[tokio::test]
async fn video_gen_send_via() {
    let (s, c) =
        ok_server(json!({"id": "task-1", "model": "cogvideox-3", "task_status": "PROCESSING"}))
            .await;
    use zai_rs::model::gen_video_async::*;
    let response = VideoGenRequest::new(zai_rs::model::gen_video_async::CogVideoX3 {})
        .with_prompt("dog")
        .send_via(&c)
        .await
        .unwrap();
    assert_eq!(response.id.as_deref(), Some("task-1"));
    assert!(response.task_status.is_some());
    assert_request(&s, "POST", "/api/paas/v4/videos/generations");
    assert_json_body(
        &s,
        json!({
            "model": "cogvideox-3",
            "prompt": "dog"
        }),
    );
    s.shutdown().await;
}

// --- Voice clone ---
#[tokio::test]
async fn voice_clone_send_via() {
    let (s, c) = ok_server(json!({
        "voice": "voice1",
        "file_id": "preview-1",
        "file_purpose": "voice-clone-output"
    }))
    .await;
    use zai_rs::model::voice_clone::*;
    let response = VoiceCloneRequest::new(
        zai_rs::model::voice_clone::GlmTtsClone {},
        "voice1",
        "hello",
        "file-1",
    )
    .send_via(&c)
    .await
    .unwrap();
    assert_eq!(response.voice.as_deref(), Some("voice1"));
    assert_eq!(response.file_id.as_deref(), Some("preview-1"));
    assert_request(&s, "POST", "/api/paas/v4/voice/clone");
    s.shutdown().await;
}

// --- Voice delete ---
#[tokio::test]
async fn voice_delete_send_via() {
    let (s, c) = ok_server(json!({
        "voice": "voice1",
        "update_time": "2026-01-01T00:00:00Z"
    }))
    .await;
    use zai_rs::model::voice_delete::*;
    let response = VoiceDeleteRequest::new("voice1")
        .send_via(&c)
        .await
        .unwrap();
    assert_eq!(response.voice.as_deref(), Some("voice1"));
    assert_request(&s, "POST", "/api/paas/v4/voice/delete");
    s.shutdown().await;
}

// --- Voice list ---
#[tokio::test]
async fn voice_list_send_via() {
    let (s, c) = ok_server(json!({
        "voice_list": [{"voice": "voice1", "voice_name": "Test voice", "voice_type": "PRIVATE"}]
    }))
    .await;
    use zai_rs::model::voice_list::*;
    let response = VoiceListRequest::new().send_via(&c).await.unwrap();
    assert_eq!(
        response.voice_list.unwrap()[0].voice.as_deref(),
        Some("voice1")
    );
    assert_request(&s, "GET", "/api/paas/v4/voice/list");
    assert!(
        s.requests()[0].query.is_none(),
        "the default voice query must not emit an empty query string"
    );
    assert_empty_body(&s);
    s.shutdown().await;
}

#[tokio::test]
async fn voice_list_query_uses_upstream_names_and_percent_encoding() {
    let (s, c) = ok_server(json!({"voice_list": []})).await;
    use zai_rs::model::voice_list::*;
    let name = "中文 %&/?=+";

    let response = VoiceListRequest::new()
        .with_query(
            VoiceListQuery::new()
                .with_voice_name(name)
                .with_voice_type(VoiceType::Private),
        )
        .send_via(&c)
        .await
        .unwrap();
    assert!(response.voice_list.unwrap().is_empty());
    assert_request(&s, "GET", "/api/paas/v4/voice/list");
    assert_empty_body(&s);

    let requests = s.requests();
    let pairs = url::form_urlencoded::parse(
        requests[0]
            .query
            .as_deref()
            .expect("voice filters must be encoded as URL query parameters")
            .as_bytes(),
    )
    .into_owned()
    .collect::<Vec<_>>();
    assert_eq!(
        pairs,
        vec![
            ("voiceName".to_string(), name.to_string()),
            ("voiceType".to_string(), "PRIVATE".to_string()),
        ]
    );
    assert!(!pairs.iter().any(|(key, _)| key.contains('_')));
    s.shutdown().await;
}

#[tokio::test]
async fn voice_list_rejects_a_blank_name_before_network_io() {
    let (s, c) = ok_server(json!({"voice_list": []})).await;
    use zai_rs::model::voice_list::*;

    let error = VoiceListRequest::new()
        .with_query(VoiceListQuery::new().with_voice_name(" \t\u{2003}"))
        .send_via(&c)
        .await
        .expect_err("a blank voice name must fail validation");

    assert_eq!(
        error.code(),
        Some(zai_rs::client::error::codes::SDK_VALIDATION)
    );
    assert!(s.requests().is_empty());
    s.shutdown().await;
}

// --- Web search ---
#[tokio::test]
async fn web_search_send_via() {
    let (s, c) = ok_server(json!({
        "id": "search-1",
        "created": 1,
        "request_id": "request-1",
        "search_intent": [],
        "search_result": []
    }))
    .await;
    use zai_rs::tool::web_search::*;
    let response = WebSearchRequest::new("rust", SearchEngine::SearchStd)
        .send_via(&c)
        .await
        .unwrap();
    assert_eq!(response.task_id(), Some("search-1"));
    assert_eq!(response.result_count(), 0);
    assert_request(&s, "POST", "/api/paas/v4/web_search");
    s.shutdown().await;
}

// --- Async chat ---
#[tokio::test]
async fn async_chat_send_via() {
    let (s, c) = ok_server(json!({"id": "task-1", "model": "glm-4.5"})).await;
    use zai_rs::model::*;
    let response = AsyncChatCompletion::new(GLM4_5_air {}, TextMessage::user("hi"))
        .send_via(&c)
        .await
        .unwrap();
    assert_eq!(response.id.as_deref(), Some("task-1"));
    assert_request(&s, "POST", "/api/paas/v4/async/chat/completions");
    assert_json_body(
        &s,
        json!({
            "model": "glm-4.5-air",
            "messages": [{"role": "user", "content": "hi"}]
        }),
    );
    s.shutdown().await;
}

// --- Error response (envelope probe exercises) ---
#[tokio::test]
async fn send_via_error_envelope() {
    let server = TestServer::start(vec![ScriptedResponse::json(
        200,
        json!({
            "code": 1302, "message": "rate limited"
        }),
    )])
    .await;
    let base = format!("{}/api/paas/v4", server.base_url);
    let c = mock_client(&base);
    use zai_rs::model::*;
    let error = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hi"))
        .send_via(&c)
        .await
        .unwrap_err();
    assert_eq!(error.code(), Some(1302));
    assert_request(&server, "POST", "/api/paas/v4/chat/completions");
    server.shutdown().await;
}

// --- HTTP 500 error ---
#[tokio::test]
async fn send_via_http_500() {
    let server = TestServer::start(vec![ScriptedResponse::json(
        500,
        json!({"error": "server"}),
    )])
    .await;
    let base = format!("{}/api/paas/v4", server.base_url);
    let c = mock_client(&base);
    use zai_rs::model::*;
    let error = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hi"))
        .send_via(&c)
        .await
        .unwrap_err();
    assert_eq!(error.code(), Some(500));
    assert_request(&server, "POST", "/api/paas/v4/chat/completions");
    server.shutdown().await;
}

// --- Knowledge endpoints (LlmApplication family) ---
fn llm_mock_client(base: &str) -> ZaiClient {
    ZaiClient::builder(KEY)
        .allow_insecure_transport(true)
        .endpoint(ApiFamily::LlmApplication, base)
        .build()
        .unwrap()
}

async fn llm_ok_server(body: serde_json::Value) -> (TestServer, ZaiClient) {
    let server = TestServer::start(vec![ScriptedResponse::json(200, body)]).await;
    let base = format!("{}/api/llm-application/open", server.base_url);
    (server, llm_mock_client(&base))
}

#[tokio::test]
async fn knowledge_list_send_via() {
    let (s, c) = llm_ok_server(json!({"code": 200, "data": {"list": [], "total": 0}})).await;
    use zai_rs::knowledge::*;
    let response = KnowledgeListRequest::new().send_via(&c).await.unwrap();
    assert_eq!(response.code, Some(200));
    assert_eq!(response.data.as_ref().unwrap().total, Some(0));
    assert_request(&s, "GET", "/api/llm-application/open/knowledge");
    let requests = s.requests();
    let pairs = url::form_urlencoded::parse(
        requests[0]
            .query
            .as_deref()
            .expect("the default knowledge pagination must be present")
            .as_bytes(),
    )
    .into_owned()
    .collect::<Vec<_>>();
    assert_eq!(
        pairs,
        vec![
            ("page".to_owned(), "1".to_owned()),
            ("size".to_owned(), "10".to_owned()),
        ]
    );
    assert_empty_body(&s);
    s.shutdown().await;
}

#[tokio::test]
async fn knowledge_data_envelope_rejects_http_200_false_successes() {
    use zai_rs::knowledge::*;

    for (case, body) in [
        (
            "missing business code",
            json!({"data": {"list": [], "total": 0}}),
        ),
        (
            "non-success business code",
            json!({"code": 0, "data": {"list": [], "total": 0}}),
        ),
        ("missing data", json!({"code": 200, "message": "ok"})),
        ("null data", json!({"code": 200, "data": null})),
    ] {
        let (server, client) = llm_ok_server(body).await;
        KnowledgeListRequest::new()
            .send_via(&client)
            .await
            .expect_err(case);
        assert_request(&server, "GET", "/api/llm-application/open/knowledge");
        server.shutdown().await;
    }
}

#[tokio::test]
async fn knowledge_operation_envelope_rejects_http_200_false_successes() {
    use zai_rs::knowledge::*;

    for (case, body) in [
        ("missing business code", json!({"message": "deleted"})),
        (
            "non-success business code",
            json!({"code": 0, "message": "deleted"}),
        ),
    ] {
        let (server, client) = llm_ok_server(body).await;
        KnowledgeDeleteRequest::new("kb1")
            .send_via(&client)
            .await
            .expect_err(case);
        assert_request(&server, "DELETE", "/api/llm-application/open/knowledge/kb1");
        server.shutdown().await;
    }
}

#[tokio::test]
async fn knowledge_list_validated_pagination_matches_the_existing_wire_shape() {
    let (server, client) =
        llm_ok_server(json!({"code": 200, "data": {"list": [], "total": 0}})).await;
    use zai_rs::{knowledge::*, pagination::PagePagination};
    KnowledgeListRequest::new()
        .try_with_pagination(PagePagination::try_new(2, 7).unwrap())
        .unwrap()
        .send_via(&client)
        .await
        .unwrap();

    let requests = server.requests();
    let pairs = url::form_urlencoded::parse(
        requests[0]
            .query
            .as_deref()
            .expect("validated knowledge pagination must be present")
            .as_bytes(),
    )
    .into_owned()
    .collect::<Vec<_>>();
    assert_eq!(
        pairs,
        vec![
            ("page".to_owned(), "2".to_owned()),
            ("size".to_owned(), "7".to_owned()),
        ]
    );
    server.shutdown().await;
}

#[tokio::test]
async fn knowledge_list_rejects_invalid_pagination_before_network_io() {
    let (s, c) = llm_ok_server(json!({"code": 200, "data": {"list": [], "total": 0}})).await;
    use zai_rs::knowledge::*;

    let error = KnowledgeListRequest::new()
        .with_query(KnowledgeListQuery::new().with_page(0))
        .send_via(&c)
        .await
        .expect_err("page zero must fail validation");

    assert_eq!(
        error.code(),
        Some(zai_rs::client::error::codes::SDK_VALIDATION)
    );
    assert!(s.requests().is_empty());
    s.shutdown().await;
}

#[tokio::test]
async fn knowledge_create_send_via() {
    let (s, c) = llm_ok_server(json!({"code": 200, "data": {"id": "kb1", "name": "test"}})).await;
    use zai_rs::knowledge::*;
    let response = KnowledgeCreateRequest::new(EmbeddingId::Embedding2, "test")
        .send_via(&c)
        .await
        .unwrap();
    assert_eq!(response.data.as_ref().unwrap().id.as_deref(), Some("kb1"));
    assert_request(&s, "POST", "/api/llm-application/open/knowledge");
    assert_json_body(
        &s,
        json!({
            "embedding_id": 3,
            "name": "test"
        }),
    );
    s.shutdown().await;
}

#[tokio::test]
async fn knowledge_capacity_send_via() {
    let (s, c) = llm_ok_server(json!({
        "code": 200,
        "data": {
            "used": {"word_num": 10, "length": 100},
            "total": {"word_num": 100, "length": 1000}
        }
    }))
    .await;
    use zai_rs::knowledge::*;
    let response = KnowledgeCapacityRequest::new().send_via(&c).await.unwrap();
    assert_eq!(
        response
            .data
            .as_ref()
            .unwrap()
            .used
            .as_ref()
            .unwrap()
            .word_num,
        Some(10)
    );
    assert_request(&s, "GET", "/api/llm-application/open/knowledge/capacity");
    s.shutdown().await;
}

#[tokio::test]
async fn knowledge_retrieve_send_via() {
    let (s, c) = llm_ok_server(json!({"code": 200, "data": {"id": "x"}})).await;
    use zai_rs::knowledge::*;
    let response = KnowledgeGetRequest::new("kb1").send_via(&c).await.unwrap();
    assert_eq!(response.data.as_ref().unwrap().id.as_deref(), Some("x"));
    assert_request(&s, "GET", "/api/llm-application/open/knowledge/kb1");
    s.shutdown().await;
}

#[tokio::test]
async fn knowledge_delete_send_via() {
    let (s, c) = llm_ok_server(json!({"code": 200, "message": "deleted"})).await;
    use zai_rs::knowledge::*;
    let response = KnowledgeDeleteRequest::new("kb1")
        .send_via(&c)
        .await
        .unwrap();
    assert_eq!(response.code, Some(200));
    assert_request(&s, "DELETE", "/api/llm-application/open/knowledge/kb1");
    s.shutdown().await;
}

#[tokio::test]
async fn knowledge_update_send_via() {
    let (s, c) = llm_ok_server(json!({"code": 200, "message": "ok"})).await;
    use zai_rs::knowledge::*;
    let response = KnowledgeUpdateRequest::new("kb1")
        .with_name("updated name")
        .send_via(&c)
        .await
        .unwrap();
    assert_eq!(response.code, Some(200));
    assert_request(&s, "PUT", "/api/llm-application/open/knowledge/kb1");
    s.shutdown().await;
}

#[tokio::test]
async fn knowledge_search_send_via() {
    let (s, c) = llm_ok_server(json!({
        "code": 200,
        "data": [{
            "text": "match",
            "score": 0.8,
            "metadata": {"doc_id": "doc1"}
        }]
    }))
    .await;
    use zai_rs::knowledge::*;
    let response = KnowledgeSearchRequest::new("kb1", "query")
        .send_via(&c)
        .await
        .unwrap();
    assert_eq!(
        response.data.as_ref().unwrap()[0].text.as_deref(),
        Some("match")
    );
    assert_eq!(
        response.data.as_ref().unwrap()[0]
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.doc_id.as_deref()),
        Some("doc1")
    );
    assert_request(&s, "POST", "/api/llm-application/open/knowledge/retrieve");
    s.shutdown().await;
}

#[tokio::test]
async fn knowledge_document_list_send_via() {
    let (s, c) = llm_ok_server(json!({
        "code": 200,
        "data": {"list": [], "total": 0}
    }))
    .await;
    use zai_rs::knowledge::*;
    let response = DocumentListRequest::new("kb1").send_via(&c).await.unwrap();
    assert_eq!(response.data.as_ref().unwrap().total, Some(0));
    assert_request(&s, "GET", "/api/llm-application/open/document");
    let requests = s.requests();
    let pairs = url::form_urlencoded::parse(
        requests[0]
            .query
            .as_deref()
            .expect("the default document-list query must be present")
            .as_bytes(),
    )
    .into_owned()
    .collect::<Vec<_>>();
    assert_eq!(
        pairs,
        vec![
            ("knowledge_id".to_owned(), "kb1".to_owned()),
            ("page".to_owned(), "1".to_owned()),
            ("size".to_owned(), "10".to_owned()),
        ]
    );
    assert_empty_body(&s);
    s.shutdown().await;
}

// --- File endpoints ---
#[tokio::test]
async fn file_list_send_via() {
    let (s, c) = ok_server(json!({"object": "list", "data": [], "has_more": false})).await;
    use zai_rs::file::*;
    let response = FileListRequest::new(FileListPurpose::Batch)
        .send_via(&c)
        .await
        .unwrap();
    assert_eq!(response.has_more, Some(false));
    assert_request(&s, "GET", "/api/paas/v4/files");
    let requests = s.requests();
    let pairs = url::form_urlencoded::parse(
        requests[0]
            .query
            .as_deref()
            .expect("the required purpose must be sent as a query parameter")
            .as_bytes(),
    )
    .into_owned()
    .collect::<Vec<_>>();
    assert_eq!(pairs, vec![("purpose".to_string(), "batch".to_string())]);
    s.shutdown().await;
}

#[tokio::test]
async fn file_list_query_preserves_enum_values_and_encodes_each_scalar_once() {
    use zai_rs::{file::*, pagination::CursorPagination};

    const CURSOR: &str = "游标 &/?=+% 空格";
    for (purpose, expected_purpose) in [
        (FileListPurpose::Batch, "batch"),
        (FileListPurpose::CodeInterpreter, "code-interpreter"),
        (FileListPurpose::Agent, "agent"),
    ] {
        let (server, client) =
            ok_server(json!({"object": "list", "data": [], "has_more": false})).await;
        let pagination = CursorPagination::new()
            .try_with_after(CURSOR)
            .unwrap()
            .try_with_limit(100)
            .unwrap();
        let query = FileListQuery::new(purpose).with_order(FileOrder::CreatedAt);
        assert!(!format!("{query:?}").contains(CURSOR));

        FileListRequest::new(purpose)
            .with_query(query)
            .try_with_pagination(pagination)
            .unwrap()
            .send_via(&client)
            .await
            .unwrap();

        assert_request(&server, "GET", "/api/paas/v4/files");
        let requests = server.requests();
        let pairs = url::form_urlencoded::parse(
            requests[0]
                .query
                .as_deref()
                .expect("file-list filters must be URL query parameters")
                .as_bytes(),
        )
        .into_owned()
        .collect::<Vec<_>>();
        assert_eq!(pairs.len(), 4, "query values must not inject extra pairs");
        let fields = pairs
            .into_iter()
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(fields.len(), 4, "query keys must be unique");
        assert_eq!(fields.get("after").map(String::as_str), Some(CURSOR));
        assert_eq!(
            fields.get("purpose").map(String::as_str),
            Some(expected_purpose)
        );
        assert_eq!(fields.get("order").map(String::as_str), Some("created_at"));
        assert_eq!(fields.get("limit").map(String::as_str), Some("100"));
        server.shutdown().await;
    }
}

#[tokio::test]
async fn file_delete_send_via() {
    let (s, c) = ok_server(json!({"id": "f1", "deleted": true})).await;
    use zai_rs::file::*;
    let response = FileDeleteRequest::new("f1").send_via(&c).await.unwrap();
    assert_eq!(response.id.as_deref(), Some("f1"));
    assert_eq!(response.deleted, Some(true));
    assert_request(&s, "DELETE", "/api/paas/v4/files/f1");
    s.shutdown().await;
}

// --- Batch endpoints ---
#[tokio::test]
async fn batch_list_send_via() {
    let (s, c) = ok_server(json!({"data": [], "object": "list", "has_more": false})).await;
    use zai_rs::batches::*;
    let response = BatchListRequest::new().send_via(&c).await.unwrap();
    assert_eq!(response.has_more, Some(false));
    assert_request(&s, "GET", "/api/paas/v4/batches");
    assert!(
        s.requests()[0].query.is_none(),
        "an empty batch query must not append a trailing query string"
    );
    s.shutdown().await;
}

#[tokio::test]
async fn batch_list_query_round_trips_cursor_and_preserves_zero_limit() {
    let (server, client) =
        ok_server(json!({"data": [], "object": "list", "has_more": false})).await;
    use zai_rs::batches::*;

    const CURSOR: &str = "游标 &/?=+% 空格";
    let query = BatchListQuery::new().with_after(CURSOR).with_limit(0);
    assert!(!format!("{query:?}").contains(CURSOR));
    BatchListRequest::new()
        .with_query(query)
        .send_via(&client)
        .await
        .unwrap();

    assert_request(&server, "GET", "/api/paas/v4/batches");
    let requests = server.requests();
    let pairs = url::form_urlencoded::parse(
        requests[0]
            .query
            .as_deref()
            .expect("batch pagination must be encoded as URL query parameters")
            .as_bytes(),
    )
    .into_owned()
    .collect::<Vec<_>>();
    assert_eq!(pairs.len(), 2, "the cursor must not inject extra pairs");
    let fields = pairs
        .into_iter()
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(fields.get("after").map(String::as_str), Some(CURSOR));
    assert_eq!(fields.get("limit").map(String::as_str), Some("0"));
    server.shutdown().await;
}

#[tokio::test]
async fn batch_list_validated_pagination_matches_the_existing_wire_shape() {
    let (server, client) =
        ok_server(json!({"data": [], "object": "list", "has_more": false})).await;
    use zai_rs::{batches::*, pagination::CursorPagination};

    const CURSOR: &str = "validated 游标 &/?=+%";
    let pagination = CursorPagination::new()
        .try_with_after(CURSOR)
        .unwrap()
        .try_with_limit(20)
        .unwrap();
    BatchListRequest::new()
        .try_with_pagination(pagination)
        .unwrap()
        .send_via(&client)
        .await
        .unwrap();

    let requests = server.requests();
    let pairs = url::form_urlencoded::parse(
        requests[0]
            .query
            .as_deref()
            .expect("validated batch pagination must be present")
            .as_bytes(),
    )
    .into_owned()
    .collect::<Vec<_>>();
    assert_eq!(
        pairs,
        vec![
            ("after".to_owned(), CURSOR.to_owned()),
            ("limit".to_owned(), "20".to_owned()),
        ]
    );
    server.shutdown().await;
}

#[tokio::test]
async fn list_query_validation_fails_before_network_io() {
    let (server, client) = ok_server(json!({"data": []})).await;

    for query in [
        zai_rs::file::FileListQuery::new(zai_rs::file::FileListPurpose::Batch).with_after(""),
        zai_rs::file::FileListQuery::new(zai_rs::file::FileListPurpose::Batch).with_after(" \t"),
        zai_rs::file::FileListQuery::new(zai_rs::file::FileListPurpose::Batch).with_limit(0),
        zai_rs::file::FileListQuery::new(zai_rs::file::FileListPurpose::Batch).with_limit(101),
    ] {
        let error = zai_rs::file::FileListRequest::new(zai_rs::file::FileListPurpose::Batch)
            .with_query(query)
            .send_via(&client)
            .await
            .expect_err("invalid file-list query must fail locally");
        assert_eq!(
            error.code(),
            Some(zai_rs::client::error::codes::SDK_VALIDATION)
        );
    }

    for query in [
        zai_rs::batches::BatchListQuery::new().with_after(""),
        zai_rs::batches::BatchListQuery::new().with_after(" \t"),
    ] {
        let error = zai_rs::batches::BatchListRequest::new()
            .with_query(query)
            .send_via(&client)
            .await
            .expect_err("invalid batch-list query must fail locally");
        assert_eq!(
            error.code(),
            Some(zai_rs::client::error::codes::SDK_VALIDATION)
        );
    }

    assert!(
        server.requests().is_empty(),
        "query validation must complete before any network I/O"
    );
    server.shutdown().await;
}

// --- OCR send_via ---
#[tokio::test]
async fn ocr_send_via() {
    let (s, c) = ok_server(json!({
        "task_id": "t1", "message": "ok", "status": "succeeded",
        "words_result_num": 1,
        "words_result": [{
            "location": {"left": 0, "top": 0, "width": 10, "height": 5},
            "words": "hello",
            "probability": {"average": 0.99, "variance": 0.01, "min": 0.95}
        }]
    }))
    .await;
    // OCR needs a real file — create a temp one
    let dir = tempfile::tempdir().unwrap();
    let img = dir.path().join("test.png");
    std::fs::write(&img, b"fake-png").unwrap();
    use zai_rs::model::ocr::*;
    let response = OcrRequest::new()
        .with_file_path(img.to_str().unwrap())
        .with_tool_type(OcrToolType::HandWrite)
        .send_via(&c)
        .await
        .unwrap();
    assert_eq!(response.task_id, "t1");
    assert_eq!(response.words_result.unwrap()[0].words, "hello");
    assert_request(&s, "POST", "/api/paas/v4/files/ocr");
    s.shutdown().await;
}

// --- Knowledge document upload URL ---
#[tokio::test]
async fn knowledge_doc_upload_url_send_via() {
    let (s, c) = llm_ok_server(json!({
        "code": 200,
        "data": {
            "successInfos": [{"documentId": "d1", "url": "https://example.com/doc.pdf"}],
            "failedInfos": []
        }
    }))
    .await;
    use zai_rs::knowledge::*;
    let body = DocumentUrlUploadBody::new("kb1").add_url("https://example.com/doc.pdf");
    let response = DocumentUrlUploadRequest::new(body)
        .send_via(&c)
        .await
        .unwrap();
    assert_eq!(
        response.data.unwrap().success_infos.unwrap()[0]
            .document_id
            .as_deref(),
        Some("d1")
    );
    assert_request(&s, "POST", "/api/llm-application/open/document/upload_url");
    assert_json_body(
        &s,
        json!({
            "upload_detail": [{
                "url": "https://example.com/doc.pdf",
                "knowledge_type": 1
            }],
            "knowledge_id": "kb1"
        }),
    );
    s.shutdown().await;
}

// --- Knowledge document list with query ---
#[tokio::test]
async fn knowledge_doc_list_with_query_send_via() {
    let (s, c) = llm_ok_server(json!({
        "code": 200,
        "data": {"list": [], "total": 0}
    }))
    .await;
    use zai_rs::{knowledge::*, pagination::PagePagination};
    let filter = "中文 %&/?=+";
    let response = DocumentListRequest::new(filter)
        .with_word(filter)
        .try_with_pagination(PagePagination::try_new(2, 5).unwrap())
        .unwrap()
        .send_via(&c)
        .await
        .unwrap();
    assert!(response.data.unwrap().list.unwrap().is_empty());
    assert_request(&s, "GET", "/api/llm-application/open/document");
    assert_empty_body(&s);

    let requests = s.requests();
    let pairs = url::form_urlencoded::parse(
        requests[0]
            .query
            .as_deref()
            .expect("document filters must be encoded as query parameters")
            .as_bytes(),
    )
    .into_owned()
    .collect::<Vec<_>>();
    assert_eq!(
        pairs,
        vec![
            ("knowledge_id".to_owned(), filter.to_owned()),
            ("page".to_owned(), "2".to_owned()),
            ("size".to_owned(), "5".to_owned()),
            ("word".to_owned(), filter.to_owned()),
        ]
    );
    s.shutdown().await;
}

#[tokio::test]
async fn document_list_rejects_blank_filters_before_network_io() {
    let (s, c) = llm_ok_server(json!({
        "code": 200,
        "data": {"list": [], "total": 0}
    }))
    .await;
    use zai_rs::knowledge::*;

    for query in [
        DocumentListQuery::new(" \t\u{2003}"),
        DocumentListQuery::new("kb1").with_word(" \t\u{2003}"),
    ] {
        let error = DocumentListRequest::new("replaced")
            .with_query(query)
            .send_via(&c)
            .await
            .expect_err("blank document filters must fail validation");
        assert_eq!(
            error.code(),
            Some(zai_rs::client::error::codes::SDK_VALIDATION)
        );
    }

    assert!(s.requests().is_empty());
    s.shutdown().await;
}

// --- Knowledge document image list ---
#[tokio::test]
async fn knowledge_doc_image_list_send_via() {
    let (s, c) = llm_ok_server(json!({
        "code": 200,
        "data": {"images": [{"text": "figure 1", "cos_url": "https://example.com/1.png"}]}
    }))
    .await;
    use zai_rs::knowledge::*;
    let response = DocumentImageListRequest::new("doc1")
        .send_via(&c)
        .await
        .unwrap();
    assert_eq!(response.data.unwrap().images.unwrap().len(), 1);
    assert_request(
        &s,
        "POST",
        "/api/llm-application/open/document/slice/image_list/doc1",
    );
    assert_empty_body(&s);
    s.shutdown().await;
}

// --- Knowledge document reembedding ---
#[tokio::test]
async fn knowledge_doc_reembedding_send_via() {
    let (s, c) = llm_ok_server(json!({"code": 200, "message": "queued"})).await;
    use zai_rs::knowledge::*;
    let response = DocumentReembedRequest::new("doc1")
        .send_via(&c)
        .await
        .unwrap();
    assert_eq!(response.code, Some(200));
    assert_request(
        &s,
        "POST",
        "/api/llm-application/open/document/embedding/doc1",
    );
    assert_empty_body(&s);
    s.shutdown().await;
}

// --- Knowledge document retrieve ---
#[tokio::test]
async fn knowledge_doc_retrieve_send_via() {
    let (s, c) =
        llm_ok_server(json!({"code": 200, "data": {"id": "doc1", "name": "test.pdf"}})).await;
    use zai_rs::knowledge::*;
    let response = DocumentGetRequest::new("doc1").send_via(&c).await.unwrap();
    assert_eq!(response.data.as_ref().unwrap().id.as_deref(), Some("doc1"));
    assert_request(&s, "GET", "/api/llm-application/open/document/doc1");
    s.shutdown().await;
}

// --- Knowledge document delete ---
#[tokio::test]
async fn knowledge_doc_delete_send_via() {
    let (s, c) = llm_ok_server(json!({"code": 200, "message": "deleted"})).await;
    use zai_rs::knowledge::*;
    let response = DocumentDeleteRequest::new("doc1")
        .send_via(&c)
        .await
        .unwrap();
    assert_eq!(response.code, Some(200));
    assert_request(&s, "DELETE", "/api/llm-application/open/document/doc1");
    s.shutdown().await;
}

// --- Services: assistant invoke ---
#[tokio::test]
async fn assistant_invoke_send_via() {
    let (s, c) = ok_server(json!({
        "id": "response-1",
        "request_id": "request-1",
        "created": 1,
        "model": "glm-4-assistant",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": "hello"},
            "finish_reason": "stop"
        }],
        "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
    }))
    .await;
    use zai_rs::services::assistants::*;
    let response =
        AssistantInvokeRequest::new(AssistantId::ChatGlm, vec![AssistantMessage::user("hi")])
            .send_via(&c)
            .await
            .unwrap();
    assert_eq!(response.id.as_deref(), Some("response-1"));
    assert_eq!(
        response.choices.as_ref().unwrap()[0]
            .message
            .as_ref()
            .unwrap()
            .content,
        Some(AssistantResponseContent::Text("hello".to_owned()))
    );
    assert_request(&s, "POST", "/api/paas/v4/assistant");
    assert_json_body(
        &s,
        json!({
            "assistant_id": "65940acff94777010aa6b796",
            "model": "glm-4-assistant",
            "messages": [{"role": "user", "content": "hi"}],
            "stream": false
        }),
    );
    s.shutdown().await;
}

// --- Services: assistant list ---
#[tokio::test]
async fn assistant_list_send_via() {
    let (s, c) = ok_server(json!({
        "success": true,
        "code": 200,
        "msg": "ok",
        "data": [{
            "assistant_id": "65940acff94777010aa6b796",
            "name": "ChatGLM"
        }]
    }))
    .await;
    use zai_rs::services::assistants::*;
    let response = AssistantListRequest::new().send_via(&c).await.unwrap();
    assert_eq!(
        response.data.as_ref().unwrap()[0].assistant_id,
        "65940acff94777010aa6b796"
    );
    assert_request(&s, "POST", "/api/paas/v4/assistant/list");
    assert_json_body(&s, json!({"assistant_id_list": []}));
    s.shutdown().await;
}

// --- Services: assistant conversation list ---
#[tokio::test]
async fn assistant_conversation_list_send_via() {
    let (s, c) = ok_server(json!({
        "success": true,
        "code": 200,
        "msg": "ok",
        "data": {
            "assistant_id": "65940acff94777010aa6b796",
            "conversation_list": [{
                "id": "conversation-1",
                "assistant_id": "65940acff94777010aa6b796"
            }],
            "has_more": false
        }
    }))
    .await;
    use zai_rs::{pagination::PagePagination, services::assistants::*};
    let response = AssistantConversationListRequest::new(AssistantId::ChatGlm)
        .try_with_pagination(PagePagination::try_new(2, 10).unwrap())
        .unwrap()
        .send_via(&c)
        .await
        .unwrap();
    assert_eq!(
        response.data.as_ref().unwrap().conversation_list[0].id,
        "conversation-1"
    );
    assert_request(&s, "POST", "/api/paas/v4/assistant/conversation/list");
    assert_json_body(
        &s,
        json!({
            "assistant_id": "65940acff94777010aa6b796",
            "page": 2,
            "page_size": 10
        }),
    );
    s.shutdown().await;
}

// --- Services: application invoke (V3 family) ---
#[tokio::test]
async fn application_invoke_send_via() {
    let server = TestServer::start(vec![ScriptedResponse::json(
        200,
        json!({
            "request_id": "invocation-1",
            "conversation_id": "conversation-1",
            "app_id": "app-1",
            "choices": [{
                "index": 0,
                "finish_reason": "stop",
                "messages": {"content": {"type": "text", "msg": "hello"}}
            }],
            "usage": [{"model": "glm-4", "nodeName": "chat", "totalTokenCount": 2}]
        }),
    )])
    .await;
    let base = format!("{}/api/llm-application/open", server.base_url);
    let c = ZaiClient::builder(KEY)
        .allow_insecure_transport(true)
        .endpoint(ApiFamily::ApplicationV3, base)
        .build()
        .unwrap();
    use zai_rs::services::applications::*;
    let response = ApplicationInvokeRequest::new(
        "app-1",
        vec![
            ApplicationInvokeMessage::new(vec![
                ApplicationInvokeContent::new("input", "hello").with_key("question"),
            ])
            .with_role("user"),
        ],
    )
    .send_via(&c)
    .await
    .unwrap();
    assert_eq!(response.request_id.as_deref(), Some("invocation-1"));
    assert_eq!(
        response.usage.as_ref().unwrap()[0].total_token_count,
        Some(2)
    );
    assert_request(
        &server,
        "POST",
        "/api/llm-application/open/v3/application/invoke",
    );
    assert_json_body(
        &server,
        json!({
            "app_id": "app-1",
            "stream": false,
            "messages": [{
                "role": "user",
                "content": [{"type": "input", "value": "hello", "key": "question"}]
            }]
        }),
    );
    server.shutdown().await;
}

// --- Services: application variables (V2 GET) ---
#[tokio::test]
async fn application_variables_send_via() {
    let server = TestServer::start(vec![ScriptedResponse::json(
        200,
        json!({
            "data": [{
                "id": "variable-1",
                "name": "temperature",
                "type": "selection_list",
                "tips": "Select a temperature",
                "allowed_values": ["low", "high"],
                "input_template": {"options": [0.5, 1.0]}
            }],
            "code": 200,
            "message": "ok",
            "timestamp": 1
        }),
    )])
    .await;
    let base = format!("{}/api/llm-application/open", server.base_url);
    let c = ZaiClient::builder(KEY)
        .allow_insecure_transport(true)
        .endpoint(ApiFamily::ApplicationV2, base)
        .build()
        .unwrap();
    use zai_rs::services::applications::*;
    let response = ApplicationVariablesRequest::new("app1")
        .send_via(&c)
        .await
        .unwrap();
    assert_eq!(response.data[0].name, "temperature");
    assert_request(
        &server,
        "GET",
        "/api/llm-application/open/v2/application/app1/variables",
    );
    server.shutdown().await;
}

// --- Services: application file upload (V2 multipart) ---
#[tokio::test]
async fn application_file_upload_send_via() {
    let server = TestServer::start(vec![ScriptedResponse::json(
        200,
        json!({
            "data": {
                "success_info": [{"file_id": "file-1", "file_name": "notes.txt"}],
                "fail_info": []
            },
            "code": 200,
            "message": "ok",
            "timestamp": 1
        }),
    )])
    .await;
    let base = format!("{}/api/llm-application/open", server.base_url);
    let client = ZaiClient::builder(KEY)
        .allow_insecure_transport(true)
        .endpoint(ApiFamily::ApplicationV2, base)
        .build()
        .unwrap();
    use zai_rs::services::applications::*;
    let response = ApplicationFileUploadRequest::new(
        "app-1",
        vec![("notes.txt".to_owned(), b"hello".to_vec())],
    )
    .with_upload_unit_id("upload-1")
    .with_file_type(2)
    .send_via(&client)
    .await
    .unwrap();

    assert_eq!(response.data.success_info[0].file_id, "file-1");
    assert_request(
        &server,
        "POST",
        "/api/llm-application/open/v2/application/file_upload",
    );
    assert_multipart_text_field(&server, "app_id", "app-1");
    assert_multipart_text_field(&server, "upload_unit_id", "upload-1");
    assert_multipart_text_field(&server, "file_type", "2");
    assert_multipart_file(
        &server,
        "files",
        "notes.txt",
        "application/octet-stream",
        b"hello",
    );
    server.shutdown().await;
}

// --- Services: application slice information (V2) ---
#[tokio::test]
async fn application_slice_info_send_via() {
    let server = TestServer::start(vec![ScriptedResponse::json(
        200,
        json!({
            "data": {"document_slices": [], "has_old_document": false},
            "code": 200,
            "message": "ok",
            "timestamp": 1
        }),
    )])
    .await;
    let base = format!("{}/api/llm-application/open", server.base_url);
    let client = ZaiClient::builder(KEY)
        .allow_insecure_transport(true)
        .endpoint(ApiFamily::ApplicationV2, base)
        .build()
        .unwrap();
    use zai_rs::services::applications::*;
    let response = ApplicationSliceInfoRequest::new("request-1", "node-1")
        .send_via(&client)
        .await
        .unwrap();

    assert!(!response.data.has_old_document);
    assert_request(
        &server,
        "POST",
        "/api/llm-application/open/v2/application/slice_info",
    );
    assert_json_body(
        &server,
        json!({"request_id": "request-1", "node_id": "node-1"}),
    );
    server.shutdown().await;
}

// --- Services: application conversation creation (V2, no body) ---
#[tokio::test]
async fn application_conversation_create_send_via() {
    let server = TestServer::start(vec![ScriptedResponse::json(
        200,
        json!({
            "data": {"conversation_id": "conversation-1"},
            "code": 200,
            "message": "ok",
            "timestamp": 1
        }),
    )])
    .await;
    let base = format!("{}/api/llm-application/open", server.base_url);
    let client = ZaiClient::builder(KEY)
        .allow_insecure_transport(true)
        .endpoint(ApiFamily::ApplicationV2, base)
        .build()
        .unwrap();
    use zai_rs::services::applications::*;
    let response = ApplicationConversationCreateRequest::new("app-1")
        .send_via(&client)
        .await
        .unwrap();

    assert_eq!(response.data.conversation_id, "conversation-1");
    assert_request(
        &server,
        "POST",
        "/api/llm-application/open/v2/application/app-1/conversation",
    );
    assert_empty_body(&server);
    server.shutdown().await;
}

// --- Services: application recommended questions (unversioned family) ---
#[tokio::test]
async fn application_history_send_via() {
    let (server, client) = llm_ok_server(json!({
        "data": {"problems": ["What can you do?"]},
        "code": 200,
        "message": "ok",
        "timestamp": 1
    }))
    .await;
    use zai_rs::services::applications::*;
    let response = ApplicationHistoryRequest::new("app-1", "conversation-1")
        .send_via(&client)
        .await
        .unwrap();

    assert_eq!(response.data.problems, ["What can you do?"]);
    assert_request(
        &server,
        "GET",
        "/api/llm-application/open/history_session_record/app-1/conversation-1",
    );
    assert_empty_body(&server);
    server.shutdown().await;
}

#[tokio::test]
async fn tools_parse_layout_send_via() {
    let (s, c) = ok_server(json!({
        "id": "layout-1",
        "created": 1,
        "model": "GLM-OCR",
        "md_results": "# Parsed text",
        "layout_details": [[{
            "index": 1,
            "label": "text",
            "bbox_2d": [0.1, 0.2, 0.8, 0.9],
            "content": "Parsed text",
            "height": 800,
            "width": 600
        }]],
        "layout_visualization": ["https://example.test/layout.png"],
        "data_info": {
            "num_pages": 1,
            "pages": [{"width": 600, "height": 800}]
        },
        "usage": {
            "prompt_tokens": 1.0,
            "completion_tokens": 2.0,
            "prompt_tokens_details": {"cached_tokens": 0.0},
            "total_tokens": 3
        },
        "request_id": "request-1"
    }))
    .await;
    use zai_rs::services::tools::*;
    let response = LayoutParsingRequest::new("https://example.test/document.pdf")
        .with_crop_images(true)
        .with_layout_visualization(true)
        .with_page_range(1, 2)
        .with_request_id("request-1")
        .with_user_id("user-1")
        .send_via(&c)
        .await
        .unwrap();
    assert_eq!(response.id, "layout-1");
    assert_eq!(response.md_results.as_deref(), Some("# Parsed text"));
    assert_eq!(
        response.layout_details.as_ref().unwrap()[0][0].label,
        LayoutLabel::Text
    );
    assert_eq!(response.data_info.as_ref().unwrap().num_pages, 1);
    assert_request(&s, "POST", "/api/paas/v4/layout_parsing");
    assert_json_body(
        &s,
        json!({
            "model": "glm-ocr",
            "file": "https://example.test/document.pdf",
            "return_crop_images": true,
            "need_layout_visualization": true,
            "start_page_id": 1,
            "end_page_id": 2,
            "request_id": "request-1",
            "user_id": "user-1"
        }),
    );
    s.shutdown().await;
}

// --- Services: tools read_document ---
#[tokio::test]
async fn tools_read_document_send_via() {
    let (s, c) = ok_server(json!({
        "id": "reader-1",
        "created": 1,
        "request_id": "request-1",
        "model": "reader",
        "reader_result": {
            "content": "read text",
            "description": "description",
            "title": "Title",
            "url": "https://example.test/page",
            "external": {
                "stylesheet": {"main": {"type": "text/css"}}
            },
            "metadata": {
                "keywords": "rust",
                "viewport": "width=device-width",
                "description": "metadata description",
                "format-detection": "telephone=no"
            }
        }
    }))
    .await;
    use zai_rs::services::tools::*;
    let response = ReaderRequest::new("https://example.test/page")
        .with_timeout_seconds(30)
        .with_cache_disabled(true)
        .with_return_format("markdown")
        .with_retained_images(true)
        .with_gfm_disabled(false)
        .with_image_data_urls(false)
        .with_image_summaries(true)
        .with_link_summaries(true)
        .send_via(&c)
        .await
        .unwrap();
    assert_eq!(
        response.reader_result.as_ref().unwrap().content.as_deref(),
        Some("read text")
    );
    assert_eq!(
        response
            .reader_result
            .as_ref()
            .unwrap()
            .metadata
            .as_ref()
            .unwrap()
            .format_detection
            .as_deref(),
        Some("telephone=no")
    );
    assert_request(&s, "POST", "/api/paas/v4/reader");
    assert_json_body(
        &s,
        json!({
            "url": "https://example.test/page",
            "timeout": 30,
            "no_cache": true,
            "return_format": "markdown",
            "retain_images": true,
            "no_gfm": false,
            "keep_img_data_url": false,
            "with_images_summary": true,
            "with_links_summary": true
        }),
    );
    s.shutdown().await;
}

// --- Services: images async generation ---
#[tokio::test]
async fn images_async_gen_send_via() {
    let (s, c) = ok_server(json!({
        "id": "task-1",
        "model": "glm-image",
        "request_id": "request-1",
        "task_status": "PROCESSING"
    }))
    .await;
    use zai_rs::services::images::*;
    let response = AsyncImageGenerationRequest::new("a cat")
        .with_quality(AsyncImageQuality::Hd)
        .with_size("1280x1280")
        .with_watermark(true)
        .with_user_id("user-1")
        .send_via(&c)
        .await
        .unwrap();
    assert_eq!(response.id.as_deref(), Some("task-1"));
    assert_eq!(response.model.as_deref(), Some("glm-image"));
    assert_eq!(response.request_id.as_deref(), Some("request-1"));
    assert!(matches!(
        response.status(),
        Some(zai_rs::model::TaskStatus::Processing)
    ));
    assert_request(&s, "POST", "/api/paas/v4/async/images/generations");
    assert_json_body(
        &s,
        json!({
            "model": "glm-image",
            "prompt": "a cat",
            "quality": "hd",
            "size": "1280x1280",
            "watermark_enabled": true,
            "user_id": "user-1"
        }),
    );
    s.shutdown().await;
}

// --- File parser create ---
#[tokio::test]
async fn file_parser_create_send_via() {
    let (s, c) = ok_server(json!({"message": "accepted", "task_id": "t1"})).await;
    let dir = tempfile::tempdir().unwrap();
    let doc = dir.path().join("test.txt");
    std::fs::write(&doc, b"hello").unwrap();
    use zai_rs::tool::file_parser_create::*;
    let response = FileParseRequest::new(&doc, ToolType::Lite)
        .unwrap()
        .with_file_type(FileType::TXT)
        .unwrap()
        .send_via(&c)
        .await
        .unwrap();
    assert_eq!(response.task_id(), Some("t1"));
    assert!(response.is_success());
    assert_request(&s, "POST", "/api/paas/v4/files/parser/create");
    assert_multipart_text_field(&s, "tool_type", "lite");
    assert_multipart_text_field(&s, "file_type", "TXT");
    assert_multipart_file(&s, "file", "test.txt", "application/octet-stream", b"hello");
    s.shutdown().await;
}

// --- File parser result ---
#[tokio::test]
async fn file_parser_result_send_via() {
    let (s, c) = ok_server(json!({
        "status": "succeeded",
        "message": "done",
        "task_id": "task-1",
        "content": "parsed content",
        "parsing_result_url": null
    }))
    .await;
    use zai_rs::tool::file_parser_result::*;
    let response = FileParseResultRequest::new("task-1")
        .get_result_via(&c, FormatType::Text)
        .await
        .unwrap();
    assert!(response.is_success());
    assert_eq!(response.content(), Some("parsed content"));
    assert_request(&s, "GET", "/api/paas/v4/files/parser/result/task-1/text");
    s.shutdown().await;
}

// --- File parse sync ---
#[tokio::test]
async fn file_parse_sync_send_via() {
    let (s, c) = ok_server(json!({
        "status": "succeeded",
        "message": "done",
        "task_id": "sync-task-1",
        "content": "sync parsed",
        "parsing_result_url": null
    }))
    .await;
    let directory = tempfile::tempdir().unwrap();
    let file_path = directory.path().join("document.txt");
    std::fs::write(&file_path, b"document body").unwrap();
    use zai_rs::file::*;
    let response = FileParseSyncRequest::new(&file_path)
        .with_file_type(FileParseSyncFileType::TXT)
        .send_via(&c)
        .await
        .unwrap();
    assert!(response.is_success());
    assert_eq!(response.task_id(), "sync-task-1");
    assert_eq!(response.content(), Some("sync parsed"));
    assert_request(&s, "POST", "/api/paas/v4/files/parser/sync");
    assert_multipart_text_field(&s, "tool_type", "prime-sync");
    assert_multipart_text_field(&s, "file_type", "TXT");
    assert_multipart_file(
        &s,
        "file",
        "document.txt",
        "application/octet-stream",
        b"document body",
    );
    s.shutdown().await;
}

#[tokio::test]
async fn file_parse_sync_rejects_a_missing_file_before_network_io() {
    let (server, client) = ok_server(json!({
        "status": "succeeded",
        "message": "unused",
        "task_id": "unused"
    }))
    .await;
    use zai_rs::file::*;
    let error = FileParseSyncRequest::new("definitely-missing-document.pdf")
        .send_via(&client)
        .await
        .unwrap_err();

    assert!(error.is_client_error());
    assert!(
        server.requests().is_empty(),
        "invalid local files must fail before any HTTP request"
    );
    server.shutdown().await;
}

// --- Unified asynchronous task results ---
#[tokio::test]
async fn async_task_get_state_send_via() {
    let (s, c) = ok_server(json!({
        "model": "glm-5.2",
        "request_id": "request-1",
        "task_status": "PROCESSING"
    }))
    .await;
    use zai_rs::model::*;
    let response = AsyncTaskGetRequest::new("task-1")
        .send_via(&c)
        .await
        .unwrap();
    assert!(
        response
            .as_state()
            .is_some_and(AsyncTaskState::is_processing)
    );
    assert_request(&s, "GET", "/api/paas/v4/async-result/task-1");
    assert_empty_body(&s);
    s.shutdown().await;
}

#[tokio::test]
async fn async_task_get_chat_result_send_via() {
    let (s, c) = ok_server(json!({
        "id": "chat-1",
        "model": "glm-5.2",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": "done"},
            "finish_reason": "stop"
        }]
    }))
    .await;
    use zai_rs::model::*;
    let response = AsyncTaskGetRequest::new("chat-1")
        .send_via(&c)
        .await
        .unwrap();
    assert_eq!(
        response
            .as_chat()
            .and_then(AsyncChatTaskResult::choices)
            .map(<[_]>::len),
        Some(1)
    );
    assert_request(&s, "GET", "/api/paas/v4/async-result/chat-1");
    assert_empty_body(&s);
    s.shutdown().await;
}

#[tokio::test]
async fn async_task_get_video_result_send_via() {
    let (s, c) = ok_server(json!({
        "model": "cogvideox-3",
        "request_id": "request-1",
        "task_status": "SUCCESS",
        "video_result": [{
            "url": "https://example.com/video.mp4",
            "cover_image_url": "https://example.com/cover.png"
        }]
    }))
    .await;
    use zai_rs::model::*;
    let response = AsyncTaskGetRequest::new("video-1")
        .send_via(&c)
        .await
        .unwrap();
    assert_eq!(
        response
            .as_video()
            .and_then(AsyncVideoTaskResult::videos)
            .map(<[_]>::len),
        Some(1)
    );
    assert_request(&s, "GET", "/api/paas/v4/async-result/video-1");
    assert_empty_body(&s);
    s.shutdown().await;
}

#[tokio::test]
async fn async_task_get_image_result_send_via() {
    let (s, c) = ok_server(json!({
        "model": "glm-image",
        "request_id": "request-1",
        "task_status": "SUCCESS",
        "image_result": [{"url": "https://example.com/image.png"}]
    }))
    .await;
    use zai_rs::model::*;
    let response = AsyncTaskGetRequest::new("image-1")
        .send_via(&c)
        .await
        .unwrap();
    assert_eq!(
        response
            .as_image()
            .and_then(AsyncImageTaskResult::images)
            .map(<[_]>::len),
        Some(1)
    );
    assert_request(&s, "GET", "/api/paas/v4/async-result/image-1");
    assert_empty_body(&s);
    s.shutdown().await;
}

#[tokio::test]
async fn async_task_get_rejects_an_empty_result() {
    let (s, c) = ok_server(json!({})).await;
    let error = zai_rs::model::AsyncTaskGetRequest::new("task-1")
        .send_via(&c)
        .await
        .unwrap_err();
    assert!(matches!(
        error.source_error(),
        zai_rs::ZaiError::JsonError(_)
    ));
    assert_request(&s, "GET", "/api/paas/v4/async-result/task-1");
    assert_empty_body(&s);
    s.shutdown().await;
}

// --- Chat coding plan ---
#[tokio::test]
async fn chat_coding_plan_send_via() {
    let server = TestServer::start(vec![ScriptedResponse::json(200, json!({
        "id": "x", "model": "glm-5.2",
        "choices": [{"index": 0, "message": {"role": "assistant", "content": "hi"}, "finish_reason": "stop"}],
        "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
    }))]).await;
    let base = format!("{}/api/coding/paas/v4", server.base_url);
    let c = ZaiClient::builder(KEY)
        .allow_insecure_transport(true)
        .endpoint(ApiFamily::CodingPaasV4, base)
        .build()
        .unwrap();
    use zai_rs::model::*;
    let response = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hi"))
        .send_via_coding_plan(&c)
        .await
        .unwrap();
    assert_eq!(response.id.as_deref(), Some("x"));
    assert_request(&server, "POST", "/api/coding/paas/v4/chat/completions");
    server.shutdown().await;
}

// --- File content download ---
#[tokio::test]
async fn file_content_send_to_via() {
    let server = TestServer::start(vec![ScriptedResponse::raw(
        200,
        "application/octet-stream",
        bytes::Bytes::from_static(b"file content"),
    )])
    .await;
    let base = format!("{}/api/paas/v4", server.base_url);
    let c = mock_client(&base);
    let temp = tempfile::tempdir().unwrap();
    let dest = temp.path().join("download.bin");
    let bytes_written = zai_rs::file::FileContentRequest::new("f1")
        .send_to_via(&c, dest.to_str().unwrap())
        .await
        .unwrap();
    assert_eq!(bytes_written, 12);
    assert_eq!(std::fs::read(&dest).unwrap(), b"file content");
    assert_request(&server, "GET", "/api/paas/v4/files/f1/content");
    server.shutdown().await;
}

// --- Batch create (POST) ---
#[tokio::test]
async fn batch_create_send_via() {
    let (s, c) = ok_server(json!({
        "id": "batch-1", "object": "batch", "status": "validating"
    }))
    .await;
    use zai_rs::batches::*;
    let response = BatchCreateRequest::new("file-1", BatchEndpoint::ChatCompletions)
        .send_via(&c)
        .await
        .unwrap();
    assert_eq!(response.id.as_deref(), Some("batch-1"));
    assert_eq!(response.status.as_deref(), Some("validating"));
    assert_request(&s, "POST", "/api/paas/v4/batches");
    assert_json_body(
        &s,
        json!({
            "input_file_id": "file-1",
            "endpoint": "/v4/chat/completions",
            "auto_delete_input_file": true
        }),
    );
    s.shutdown().await;
}

// --- Batch retrieve (GET by id) ---
#[tokio::test]
async fn batch_retrieve_send_via() {
    let (s, c) = ok_server(json!({
        "id": "batch-1", "object": "batch", "status": "completed"
    }))
    .await;
    use zai_rs::batches::*;
    let response = BatchGetRequest::new("batch-1").send_via(&c).await.unwrap();
    assert_eq!(response.id.as_deref(), Some("batch-1"));
    assert_eq!(response.status.as_deref(), Some("completed"));
    assert_request(&s, "GET", "/api/paas/v4/batches/batch-1");
    s.shutdown().await;
}

// --- Batch cancel (POST to cancel path) ---
#[tokio::test]
async fn batch_cancel_send_via() {
    let (s, c) = ok_server(json!({
        "id": "batch-1", "object": "batch", "status": "cancelled"
    }))
    .await;
    use zai_rs::batches::*;
    let response = BatchCancelRequest::new("batch-1")
        .send_via(&c)
        .await
        .unwrap();
    assert_eq!(response.id.as_deref(), Some("batch-1"));
    assert_eq!(response.status.as_deref(), Some("cancelled"));
    assert_request(&s, "POST", "/api/paas/v4/batches/batch-1/cancel");
    assert_empty_body(&s);
    s.shutdown().await;
}

#[tokio::test]
async fn batch_cancel_rejects_blank_id_before_network_io() {
    let (server, client) = ok_server(json!({})).await;

    let error = zai_rs::batches::BatchCancelRequest::new("   ")
        .send_via(&client)
        .await
        .expect_err("a blank path identifier must be rejected locally");

    assert_eq!(
        error.code(),
        Some(zai_rs::client::error::codes::SDK_VALIDATION)
    );
    assert!(server.requests().is_empty());
    server.shutdown().await;
}

// --- Services: application file_stats (POST, ApplicationV2 family) ---
#[tokio::test]
async fn application_file_stats_send_via() {
    let server = TestServer::start(vec![ScriptedResponse::json(
        200,
        json!({
            "data": [{"file_id": "file-1", "code": 0, "msg": "parsed"}],
            "code": 200,
            "message": "ok",
            "timestamp": 1
        }),
    )])
    .await;
    let base = format!("{}/api/llm-application/open", server.base_url);
    let c = ZaiClient::builder(KEY)
        .allow_insecure_transport(true)
        .endpoint(ApiFamily::ApplicationV2, base)
        .build()
        .unwrap();
    use zai_rs::services::applications::*;
    let response = ApplicationFileStatsRequest::new("app-1", vec!["file-1".to_owned()])
        .send_via(&c)
        .await
        .unwrap();
    assert_eq!(response.data[0].file_id, "file-1");
    assert_request(
        &server,
        "POST",
        "/api/llm-application/open/v2/application/file_stat",
    );
    assert_json_body(&server, json!({"app_id": "app-1", "file_ids": ["file-1"]}));
    server.shutdown().await;
}

// --- TTS send_via ---
#[tokio::test]
async fn tts_send_via() {
    let server = TestServer::start(vec![ScriptedResponse::raw(
        200,
        "audio/pcm",
        bytes::Bytes::from_static(b"fake-audio-data"),
    )])
    .await;
    let base = format!("{}/api/paas/v4", server.base_url);
    let c = mock_client(&base);
    use zai_rs::model::text_to_audio::*;
    let audio = TextToAudioRequest::new(GlmTts {})
        .with_input("hello")
        .send_via(&c)
        .await
        .unwrap();
    assert_eq!(audio.as_ref(), b"fake-audio-data");
    assert_request(&server, "POST", "/api/paas/v4/audio/speech");
    server.shutdown().await;
}

// --- ASR send_via ---
#[tokio::test]
async fn asr_send_via() {
    let server = TestServer::start(vec![ScriptedResponse::json(
        200,
        json!({
            "id": "asr-1", "model": "glm-asr-2512", "text": "transcribed text"
        }),
    )])
    .await;
    let base = format!("{}/api/paas/v4", server.base_url);
    let c = mock_client(&base);
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("test.wav");
    std::fs::write(&wav, b"fake-wav").unwrap();
    use zai_rs::model::audio_to_text::*;
    let response = AudioToTextRequest::new(GlmAsr {})
        .with_file_path(wav.to_str().unwrap())
        .send_via(&c)
        .await
        .unwrap();
    assert_eq!(response.id, "asr-1");
    assert_eq!(response.text, "transcribed text");
    assert_request(&server, "POST", "/api/paas/v4/audio/transcriptions");
    server.shutdown().await;
}

// --- File upload multipart send_via ---
#[tokio::test]
async fn file_upload_send_via() {
    let server = TestServer::start(vec![ScriptedResponse::json(
        200,
        json!({
            "id": "file-1", "object": "file", "bytes": 5, "filename": "test.txt", "purpose": "agent"
        }),
    )])
    .await;
    let base = format!("{}/api/paas/v4", server.base_url);
    let c = mock_client(&base);
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("test.txt");
    std::fs::write(&f, b"hello").unwrap();
    use zai_rs::file::*;
    let response = FileUploadRequest::new(FileUploadPurpose::Agent, f.to_str().unwrap())
        .send_via(&c)
        .await
        .unwrap();
    assert_eq!(response.id.as_deref(), Some("file-1"));
    assert_eq!(response.filename.as_deref(), Some("test.txt"));
    assert_request(&server, "POST", "/api/paas/v4/files");
    assert_multipart_text_field(&server, "purpose", "agent");
    assert_multipart_file(
        &server,
        "file",
        "test.txt",
        "application/octet-stream",
        b"hello",
    );
    server.shutdown().await;
}

#[tokio::test]
async fn file_upload_rejects_unsafe_file_parts_before_network_io() {
    use zai_rs::file::{FileUploadPurpose, FileUploadRequest};

    // Exceed the SDK's bounded multipart part limit without importing private
    // transport implementation constants.
    const OVERSIZED_MULTIPART_BYTES: u64 = 128 * 1024 * 1024 + 1;

    let server = TestServer::start(Vec::new()).await;
    let base = format!("{}/api/paas/v4", server.base_url);
    let client = mock_client(&base);
    let dir = tempfile::tempdir().unwrap();

    FileUploadRequest::new(FileUploadPurpose::Agent, dir.path())
        .send_via(&client)
        .await
        .unwrap_err();

    let oversized = dir.path().join("oversized.bin");
    std::fs::File::create(&oversized)
        .unwrap()
        .set_len(OVERSIZED_MULTIPART_BYTES)
        .unwrap();
    FileUploadRequest::new(FileUploadPurpose::Agent, &oversized)
        .send_via(&client)
        .await
        .unwrap_err();

    let valid = dir.path().join("valid.txt");
    std::fs::write(&valid, b"safe").unwrap();
    FileUploadRequest::new(FileUploadPurpose::Agent, &valid)
        .with_file_name("../escape.txt")
        .send_via(&client)
        .await
        .unwrap_err();
    FileUploadRequest::new(FileUploadPurpose::Agent, &valid)
        .with_content_type("not a mime")
        .send_via(&client)
        .await
        .unwrap_err();

    #[cfg(unix)]
    {
        let symlink = dir.path().join("link.txt");
        std::os::unix::fs::symlink(&valid, &symlink).unwrap();
        FileUploadRequest::new(FileUploadPurpose::Agent, symlink)
            .send_via(&client)
            .await
            .unwrap_err();
    }

    assert!(
        server.requests().is_empty(),
        "invalid multipart metadata must be rejected before network I/O"
    );
    server.shutdown().await;
}

// --- Knowledge document upload file send_via ---
#[tokio::test]
async fn knowledge_doc_upload_file_send_via() {
    let server = TestServer::start(vec![ScriptedResponse::json(
        200,
        json!({
            "code": 200,
            "data": {
                "successInfos": [{"documentId": "d1", "fileName": "test.pdf"}],
                "failedInfos": []
            }
        }),
    )])
    .await;
    let base = format!("{}/api/llm-application/open", server.base_url);
    let c = ZaiClient::builder(KEY)
        .allow_insecure_transport(true)
        .endpoint(ApiFamily::LlmApplication, base)
        .build()
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("test.pdf");
    std::fs::write(&f, b"fake-pdf").unwrap();
    use zai_rs::knowledge::*;
    let response = DocumentUploadRequest::new("kb-1")
        .add_file_path(f)
        .send_via(&c)
        .await
        .unwrap();
    assert_eq!(
        response.data.unwrap().success_infos.unwrap()[0]
            .document_id
            .as_deref(),
        Some("d1")
    );
    assert_request(
        &server,
        "POST",
        "/api/llm-application/open/document/upload_document/kb-1",
    );
    assert_multipart_file(&server, "files", "test.pdf", "application/pdf", b"fake-pdf");
    server.shutdown().await;
}
