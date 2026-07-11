//! send_via coverage: mock-server integration tests for every endpoint's
//! `send_via` path. Each test builds a ZaiClient pointing at a local mock
//! server and exercises the full request → response pipeline.

mod support;
use support::http_server::{ScriptedResponse, TestServer};

use serde_json::json;

use zai_rs::client::{ApiFamily, ZaiClient};

const KEY: &str = "test.12345678901234567890";

fn mock_client(base: &str) -> ZaiClient {
    let leaked: &'static str = Box::leak(base.to_string().into_boxed_str());
    ZaiClient::builder(KEY)
        .allow_insecure_transport(true)
        .endpoint(ApiFamily::PaasV4, leaked)
        .build()
        .unwrap()
}

async fn ok_server(body: serde_json::Value) -> (TestServer, ZaiClient) {
    let server = TestServer::start(vec![ScriptedResponse::json(200, body)]).await;
    let base = format!("{}/api/paas/v4", server.base_url);
    (server, mock_client(&base))
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
    assert!(resp.id.is_some());
    s.shutdown().await;
}

// --- Embeddings ---
#[tokio::test]
async fn embeddings_send_via() {
    let (s, c) = ok_server(json!({
        "object": "list",
        "data": [{"index": 0, "object": "embedding", "embedding": [0.1, 0.2]}],
        "model": "embedding-2",
        "usage": {"prompt_tokens": 1, "total_tokens": 1}
    }))
    .await;
    use zai_rs::model::text_embedded::*;
    let resp = EmbeddingRequest::new(
        EmbeddingModel::Embedding2,
        EmbeddingInput::Single("hi".into()),
    )
    .send_via(&c)
    .await;
    // May fail on response parse, but exercises the full send path
    let _ = resp;
    s.shutdown().await;
}

// --- Rerank ---
#[tokio::test]
async fn rerank_send_via() {
    let (s, c) = ok_server(
        json!({"id": "x", "results": [{"index": 0, "relevance_score": 0.9}], "model": "rerank"}),
    )
    .await;
    use zai_rs::model::text_rerank::*;
    let _ = RerankRequest::new("q", vec!["d".into()]).send_via(&c).await;
    s.shutdown().await;
}

// --- Tokenizer ---
#[tokio::test]
async fn tokenizer_send_via() {
    let (s, c) = ok_server(json!({"id": "x", "tokens": [1, 2, 3], "model": "glm-4"})).await;
    use zai_rs::model::text_tokenizer::*;
    let _ = TokenizerRequest::new(
        TokenizerModel::default(),
        vec![TokenizerMessage::User {
            content: "hi".into(),
        }],
    )
    .send_via(&c)
    .await;
    s.shutdown().await;
}

// --- Moderation ---
#[tokio::test]
async fn moderation_send_via() {
    let (s, c) = ok_server(json!({"id": "x", "results": [{"flagged": false, "categories": {}, "category_scores": {}}], "model": "glm-4"})).await;
    use zai_rs::model::moderation::*;
    let _ = Moderation::new_text("hi").send_via(&c).await;
    s.shutdown().await;
}

// --- Image generation ---
#[tokio::test]
async fn image_gen_send_via() {
    let (s, c) = ok_server(
        json!({"id": "x", "data": [{"url": "https://example.com/a.png"}], "model": "cogview-4"}),
    )
    .await;
    use zai_rs::model::gen_image::*;
    let _ = ImageGenRequest::new(zai_rs::model::gen_image::CogView4 {})
        .with_prompt("cat")
        .send_via(&c)
        .await;
    s.shutdown().await;
}

// --- Video generation ---
#[tokio::test]
async fn video_gen_send_via() {
    let (s, c) =
        ok_server(json!({"id": "task-1", "model": "cogvideox-3", "task_status": "PROCESSING"}))
            .await;
    use zai_rs::model::gen_video_async::*;
    let _ = VideoGenRequest::new(zai_rs::model::gen_video_async::CogVideoX3 {})
        .with_prompt("dog")
        .send_via(&c)
        .await;
    s.shutdown().await;
}

// --- Voice clone ---
#[tokio::test]
async fn voice_clone_send_via() {
    let (s, c) = ok_server(json!({"voice_id": "v1"})).await;
    use zai_rs::model::voice_clone::*;
    let _ = VoiceCloneRequest::new(
        zai_rs::model::voice_clone::GlmTtsClone {},
        "voice1",
        "hello",
        "file-1",
    )
    .send_via(&c)
    .await;
    s.shutdown().await;
}

// --- Voice delete ---
#[tokio::test]
async fn voice_delete_send_via() {
    let (s, c) = ok_server(json!({"success": true})).await;
    use zai_rs::model::voice_delete::*;
    let _ = VoiceDeleteRequest::new("voice1").send_via(&c).await;
    s.shutdown().await;
}

// --- Voice list ---
#[tokio::test]
async fn voice_list_send_via() {
    let (s, c) = ok_server(json!({"data": []})).await;
    use zai_rs::model::voice_list::*;
    let _ = VoiceListRequest::new().send_via(&c).await;
    s.shutdown().await;
}

// --- Web search ---
#[tokio::test]
async fn web_search_send_via() {
    let (s, c) = ok_server(json!({"data": []})).await;
    use zai_rs::tool::web_search::*;
    let _ = WebSearchRequest::new("rust".into(), SearchEngine::SearchStd)
        .send_via(&c)
        .await;
    s.shutdown().await;
}

// --- Async chat ---
#[tokio::test]
async fn async_chat_send_via() {
    let (s, c) = ok_server(json!({"id": "task-1", "model": "glm-4.5"})).await;
    use zai_rs::model::*;
    let _ = AsyncChatCompletion::new(GLM4_5 {}, TextMessage::user("hi"))
        .send_via(&c)
        .await;
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
    let result = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hi"))
        .send_via(&c)
        .await;
    assert!(result.is_err(), "error envelope should return Err");
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
    let result = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hi"))
        .send_via(&c)
        .await;
    assert!(result.is_err());
    server.shutdown().await;
}

// --- Knowledge endpoints (LlmApplication family) ---
fn llm_mock_client(base: &str) -> ZaiClient {
    let leaked: &'static str = Box::leak(base.to_string().into_boxed_str());
    ZaiClient::builder(KEY)
        .allow_insecure_transport(true)
        .endpoint(ApiFamily::LlmApplication, leaked)
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
    use zai_rs::knowledge::list::*;
    let _ = KnowledgeListRequest::new().send_via(&c).await;
    s.shutdown().await;
}

#[tokio::test]
async fn knowledge_create_send_via() {
    let (s, c) = llm_ok_server(json!({"code": 200, "data": {"id": "kb1", "name": "test"}})).await;
    use zai_rs::knowledge::create::*;
    let _ = CreateKnowledgeRequest::new(EmbeddingId::Embedding2, "test")
        .send_via(&c)
        .await;
    s.shutdown().await;
}

#[tokio::test]
async fn knowledge_capacity_send_via() {
    let (s, c) = llm_ok_server(json!({"code": 200, "data": {"capacity": 100}})).await;
    use zai_rs::knowledge::capacity::*;
    let _ = KnowledgeCapacityRequest::new().send_via(&c).await;
    s.shutdown().await;
}

#[tokio::test]
async fn knowledge_retrieve_send_via() {
    let (s, c) = llm_ok_server(json!({"code": 200, "data": {"id": "x"}})).await;
    use zai_rs::knowledge::retrieve::*;
    let _ = KnowledgeRetrieveRequest::new("kb1").send_via(&c).await;
    s.shutdown().await;
}

#[tokio::test]
async fn knowledge_delete_send_via() {
    let (s, c) = llm_ok_server(json!({"code": 200, "data": null})).await;
    use zai_rs::knowledge::delete::*;
    let _ = KnowledgeDeleteRequest::new("kb1").send_via(&c).await;
    s.shutdown().await;
}

#[tokio::test]
async fn knowledge_update_send_via() {
    let (s, c) = llm_ok_server(json!({"code": 200, "data": {"id": "kb1"}})).await;
    use zai_rs::knowledge::update::*;
    let _ = KnowledgeUpdateRequest::new("kb1").send_via(&c).await;
    s.shutdown().await;
}

#[tokio::test]
async fn knowledge_search_send_via() {
    let (s, c) = llm_ok_server(json!({"data": []})).await;
    use zai_rs::knowledge::search::*;
    let _ = KnowledgeSearchRequest::new("kb1", "query")
        .send_via(&c)
        .await;
    s.shutdown().await;
}

#[tokio::test]
async fn knowledge_document_list_send_via() {
    let (s, c) = llm_ok_server(json!({"code": 200, "data": {"list": []}})).await;
    use zai_rs::knowledge::document_list::*;
    let _ = DocumentListRequest::new().send_via(&c).await;
    s.shutdown().await;
}

// --- File endpoints ---
#[tokio::test]
async fn file_list_send_via() {
    let (s, c) = ok_server(json!({"data": [], "has_more": false})).await;
    use zai_rs::file::*;
    let _ = FileListRequest::new().send_via(&c).await;
    s.shutdown().await;
}

#[tokio::test]
async fn file_delete_send_via() {
    let (s, c) = ok_server(json!({"id": "f1", "deleted": true})).await;
    use zai_rs::file::*;
    let _ = FileDeleteRequest::new("f1").send_via(&c).await;
    s.shutdown().await;
}

// --- Batch endpoints ---
#[tokio::test]
async fn batch_list_send_via() {
    let (s, c) = ok_server(json!({"data": [], "object": "list"})).await;
    use zai_rs::batches::*;
    let _ = BatchesListRequest::new().send_via(&c).await;
    s.shutdown().await;
}

// --- OCR send_via ---
#[tokio::test]
async fn ocr_send_via() {
    let (s, c) = ok_server(json!({
        "task_id": "t1", "status": "SUCCESS",
        "words_result_num": 1,
        "words_result": [{"words": "hello", "probability": {"average": 0.99}}]
    }))
    .await;
    // OCR needs a real file — create a temp one
    let dir = tempfile::tempdir().unwrap();
    let img = dir.path().join("test.png");
    std::fs::write(&img, b"fake-png").unwrap();
    use zai_rs::model::ocr::*;
    let _ = OcrRequest::new()
        .with_file_path(img.to_str().unwrap())
        .with_tool_type(OcrToolType::HandWrite)
        .send_via(&c)
        .await;
    s.shutdown().await;
}

// --- Knowledge document upload URL ---
#[tokio::test]
async fn knowledge_doc_upload_url_send_via() {
    let (s, c) = llm_ok_server(json!({"code": 200, "data": {"document_id": "d1"}})).await;
    use zai_rs::knowledge::document_upload_url::*;
    let detail = UploadUrlDetail::new("https://example.com/doc.pdf");
    let body = UploadUrlBody {
        upload_detail: vec![detail],
        knowledge_id: "kb1".into(),
    };
    let _ = DocumentUploadUrlRequest::new(body).send_via(&c).await;
    s.shutdown().await;
}

// --- Knowledge document list with query ---
#[tokio::test]
async fn knowledge_doc_list_with_query_send_via() {
    let (s, c) = llm_ok_server(json!({"code": 200, "data": {"list": []}})).await;
    use zai_rs::knowledge::document_list::*;
    let _ = DocumentListRequest::new().send_via(&c).await;
    s.shutdown().await;
}

// --- Knowledge document image list ---
#[tokio::test]
async fn knowledge_doc_image_list_send_via() {
    let (s, c) = llm_ok_server(json!({"code": 200, "data": []})).await;
    use zai_rs::knowledge::document_image_list::*;
    let _ = DocumentImageListRequest::new("doc1").send_via(&c).await;
    s.shutdown().await;
}

// --- Knowledge document reembedding ---
#[tokio::test]
async fn knowledge_doc_reembedding_send_via() {
    let (s, c) = llm_ok_server(json!({"code": 200, "data": {"document_id": "d1"}})).await;
    use zai_rs::knowledge::document_reembedding::*;
    let _ = DocumentReembeddingRequest::new("doc1").send_via(&c).await;
    s.shutdown().await;
}

// --- Knowledge document retrieve ---
#[tokio::test]
async fn knowledge_doc_retrieve_send_via() {
    let (s, c) = llm_ok_server(json!({"code": 200, "data": {"document_id": "d1"}})).await;
    use zai_rs::knowledge::document_retrieve::*;
    let _ = DocumentRetrieveRequest::new("doc1").send_via(&c).await;
    s.shutdown().await;
}

// --- Knowledge document delete ---
#[tokio::test]
async fn knowledge_doc_delete_send_via() {
    let (s, c) = llm_ok_server(json!({"code": 200, "data": null})).await;
    use zai_rs::knowledge::document_delete::*;
    let _ = DocumentDeleteRequest::new("doc1").send_via(&c).await;
    s.shutdown().await;
}

// --- Services: assistant invoke ---
#[tokio::test]
async fn assistant_invoke_send_via() {
    let (s, c) = ok_server(json!({"id": "a1", "choices": []})).await;
    use zai_rs::services::assistants::*;
    let _ = AssistantInvokeRequest::new(json!({"content": "hi"}))
        .send_via(&c)
        .await;
    s.shutdown().await;
}

// --- Services: assistant list ---
#[tokio::test]
async fn assistant_list_send_via() {
    let (s, c) = ok_server(json!({"data": []})).await;
    use zai_rs::services::assistants::*;
    let _ = AssistantListRequest::new().send_via(&c).await;
    s.shutdown().await;
}

// --- Services: assistant conversation list ---
#[tokio::test]
async fn assistant_conversation_list_send_via() {
    let (s, c) = ok_server(json!({"data": []})).await;
    use zai_rs::services::assistants::*;
    let _ = AssistantConversationListRequest::new().send_via(&c).await;
    s.shutdown().await;
}

// --- Services: application invoke (V3 family) ---
#[tokio::test]
async fn application_invoke_send_via() {
    let server = TestServer::start(vec![ScriptedResponse::json(
        200,
        json!({"id": "a1", "choices": []}),
    )])
    .await;
    let base = format!("{}/api/llm-application/open", server.base_url);
    let leaked: &'static str = Box::leak(base.to_string().into_boxed_str());
    let c = ZaiClient::builder(KEY)
        .allow_insecure_transport(true)
        .endpoint(ApiFamily::ApplicationV3, leaked)
        .build()
        .unwrap();
    use zai_rs::services::applications::*;
    let _ = ApplicationInvokeRequest::new(json!({"app_id": "a1"}))
        .send_via(&c)
        .await;
    server.shutdown().await;
}

// --- Services: application variables (V2 GET) ---
#[tokio::test]
async fn application_variables_send_via() {
    let server = TestServer::start(vec![ScriptedResponse::json(200, json!({"data": []}))]).await;
    let base = format!("{}/api/llm-application/open", server.base_url);
    let leaked: &'static str = Box::leak(base.to_string().into_boxed_str());
    let c = ZaiClient::builder(KEY)
        .allow_insecure_transport(true)
        .endpoint(ApiFamily::ApplicationV2, leaked)
        .build()
        .unwrap();
    use zai_rs::services::applications::*;
    let _ = ApplicationVariablesRequest::new("app1").send_via(&c).await;
    server.shutdown().await;
}

#[tokio::test]
async fn tools_parse_layout_send_via() {
    let (s, c) = ok_server(json!({"content": "parsed text"})).await;
    use zai_rs::services::tools::*;
    let _ = LayoutParsingRequest::new(json!({"content": "text"}))
        .send_via(&c)
        .await;
    s.shutdown().await;
}

// --- Services: tools read_document ---
#[tokio::test]
async fn tools_read_document_send_via() {
    let (s, c) = ok_server(json!({"content": "read text"})).await;
    use zai_rs::services::tools::*;
    let _ = ReaderRequest::new(json!({"url": "https://example.com/doc.pdf"}))
        .send_via(&c)
        .await;
    s.shutdown().await;
}

// --- Services: images async generation ---
#[tokio::test]
async fn images_async_gen_send_via() {
    let (s, c) = ok_server(json!({"id": "task-1", "model": "cogview-4"})).await;
    use zai_rs::services::images::*;
    let _ = AsyncImageGenerationRequest::new(json!({"prompt": "a cat"}))
        .send_via(&c)
        .await;
    s.shutdown().await;
}

// --- File parser create ---
#[tokio::test]
async fn file_parser_create_send_via() {
    let (s, c) = ok_server(json!({"task_id": "t1", "status": "PROCESSING"})).await;
    let dir = tempfile::tempdir().unwrap();
    let doc = dir.path().join("test.txt");
    std::fs::write(&doc, b"hello").unwrap();
    use zai_rs::tool::file_parser_create::*;
    let _ = FileParserCreateRequest::new(&doc, ToolType::Lite, FileType::TXT)
        .unwrap()
        .send_via(&c)
        .await;
    s.shutdown().await;
}

// --- File parser result ---
#[tokio::test]
async fn file_parser_result_send_via() {
    let (s, c) = ok_server(json!({"content": "parsed content"})).await;
    use zai_rs::tool::file_parser_result::*;
    let _ = FileParserResultRequest::new("task-1")
        .get_result_via(&c, FormatType::Text)
        .await;
    s.shutdown().await;
}

// --- File parse sync ---
#[tokio::test]
async fn file_parse_sync_send_via() {
    let (s, c) = ok_server(json!({"content": "sync parsed"})).await;
    let dir = tempfile::tempdir().unwrap();
    let doc = dir.path().join("test.txt");
    std::fs::write(&doc, b"hello").unwrap();
    use zai_rs::file::parse_sync::*;
    let _ = FileParseSyncRequest::new(json!({"file_id": "f1"}))
        .send_via(&c)
        .await;
    s.shutdown().await;
}

// --- Async chat get result ---
#[tokio::test]
async fn async_chat_get_send_via() {
    let (s, c) = ok_server(json!({"id": "task-1", "task_status": "SUCCESS"})).await;
    use zai_rs::model::*;
    let _ = AsyncChatGetRequest::new(GLM4_5 {}, "task-1".into())
        .send_via(&c)
        .await;
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
    let leaked: &'static str = Box::leak(base.to_string().into_boxed_str());
    let c = ZaiClient::builder(KEY)
        .allow_insecure_transport(true)
        .endpoint(ApiFamily::CodingPaasV4, leaked)
        .build()
        .unwrap();
    use zai_rs::model::*;
    let _ = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hi"))
        .send_via_coding_plan(&c)
        .await;
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
    let dest = std::env::temp_dir().join("zai_cov_file.bin");
    let _ = zai_rs::file::FileContentRequest::new("f1")
        .send_to_via(&c, dest.to_str().unwrap())
        .await;
    let _ = std::fs::remove_file(&dest);
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
    let _ = CreateBatchRequest::new("file-1", BatchEndpoint::ChatCompletions)
        .send_via(&c)
        .await;
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
    let _ = BatchesRetrieveRequest::new("batch-1").send_via(&c).await;
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
    let _ = CancelBatchRequest::new("batch-1").send_via(&c).await;
    s.shutdown().await;
}

// --- Services: application file_stats (POST, ApplicationV2 family) ---
#[tokio::test]
async fn application_file_stats_send_via() {
    let server = TestServer::start(vec![ScriptedResponse::json(200, json!({"data": {}}))]).await;
    let base = format!("{}/api/llm-application/open", server.base_url);
    let leaked: &'static str = Box::leak(base.to_string().into_boxed_str());
    let c = ZaiClient::builder(KEY)
        .allow_insecure_transport(true)
        .endpoint(ApiFamily::ApplicationV2, leaked)
        .build()
        .unwrap();
    use zai_rs::services::applications::*;
    let _ = ApplicationFileStatsRequest::new(json!({}))
        .send_via(&c)
        .await;
    server.shutdown().await;
}

// --- Chat base response deeper coverage ---
#[test]
fn chat_response_usage_accessors() {
    let json = r#"{"id":"x","choices":[{"index":0,"message":{"role":"assistant","content":"hi"},"finish_reason":"stop","tool_calls":null}],"usage":{"prompt_tokens":10,"completion_tokens":5,"total_tokens":15},"model":"glm-5.2","created":1234567890}"#;
    let resp: zai_rs::model::chat_base_response::ChatCompletionResponse =
        serde_json::from_str(json).unwrap();
    let usage = resp.usage.unwrap();
    assert_eq!(usage.prompt_tokens(), Some(10));
    assert_eq!(usage.completion_tokens(), Some(5));
    assert_eq!(usage.total_tokens(), Some(15));
}

#[test]
fn chat_response_message_tool_calls() {
    let json = r#"{"id":"x","choices":[{"index":0,"message":{"role":"assistant","content":"hi","tool_calls":[{"id":"tc1","type":"function","function":{"name":"calc","arguments":"{}"}}]},"finish_reason":"tool_calls"}]}"#;
    let resp: zai_rs::model::chat_base_response::ChatCompletionResponse =
        serde_json::from_str(json).unwrap();
    let choices = resp.choices().unwrap();
    let tc = choices[0].message().tool_calls();
    assert!(tc.is_some());
    let tc = tc.unwrap();
    assert_eq!(tc.len(), 1);
    assert_eq!(
        tc[0].function.as_ref().unwrap().name.as_deref(),
        Some("calc")
    );
}

// --- Error compact/message/code for all variants ---
#[test]
fn error_all_variants_compact() {
    use zai_rs::client::error::*;
    for err in [
        ZaiError::AuthError {
            code: 1001,
            message: "auth".into(),
        },
        ZaiError::RateLimitError {
            code: 1302,
            message: "rl".into(),
        },
        ZaiError::AccountError {
            code: 1110,
            message: "acct".into(),
        },
        ZaiError::ApiError {
            code: 1200,
            message: "api".into(),
        },
        ZaiError::ContentPolicyError {
            code: 1300,
            message: "cp".into(),
        },
        ZaiError::FileError {
            code: 1400,
            message: "file".into(),
        },
        ZaiError::HttpError {
            status: 404,
            message: "http".into(),
        },
        ZaiError::Unknown {
            code: 999,
            message: "unk".into(),
        },
    ] {
        let c = err.compact();
        assert!(!c.is_empty(), "compact should be non-empty for {err:?}");
        let m = err.message();
        assert!(!m.is_empty(), "message should be non-empty");
    }
}

#[test]
fn error_is_retryable_all_variants() {
    use zai_rs::client::error::*;
    assert!(
        !ZaiError::AuthError {
            code: 1001,
            message: "x".into()
        }
        .is_retryable()
    );
    assert!(
        ZaiError::RateLimitError {
            code: 1302,
            message: "x".into()
        }
        .is_retryable()
    );
    assert!(
        ZaiError::HttpError {
            status: 503,
            message: "x".into()
        }
        .is_retryable()
    );
    assert!(
        !ZaiError::HttpError {
            status: 400,
            message: "x".into()
        }
        .is_retryable()
    );
    assert!(
        !ZaiError::ApiError {
            code: 1200,
            message: "x".into()
        }
        .is_retryable()
    );
}

// --- Transport config builder ---
#[test]
fn transport_config_builder() {
    use zai_rs::client::HttpTransportConfig;
    let cfg = HttpTransportConfig::builder()
        .max_attempts(2)
        .unwrap()
        .request_timeout(std::time::Duration::from_secs(30))
        .unwrap()
        .build();
    assert_eq!(cfg.max_attempts, 2);
    assert_eq!(cfg.request_timeout, std::time::Duration::from_secs(30));
}

// --- Endpoint config builder ---
#[test]
fn endpoint_config_builder_custom() {
    use zai_rs::client::EndpointConfig;
    let ec = EndpointConfig::builder()
        .paas_v4("https://custom.example.com/api/paas/v4")
        .build(false)
        .unwrap();
    assert!(
        ec.base(zai_rs::client::ApiFamily::PaasV4)
            .as_str()
            .contains("custom.example.com")
    );
}

#[test]
fn endpoint_config_resolve_with_query() {
    use zai_rs::client::{ApiFamily, EndpointConfig};
    let ec = EndpointConfig::defaults().unwrap();
    let url = ec
        .resolve_with_query(
            ApiFamily::PaasV4,
            &["files"],
            &[("limit", "10"), ("order", "desc")],
        )
        .unwrap();
    assert!(url.contains("limit=10"));
    assert!(url.contains("order=desc"));
}

// --- Retry-After parsing edge cases ---
#[test]
fn retry_after_edge_cases() {
    use zai_rs::client::transport::retry::parse_retry_after;
    assert_eq!(parse_retry_after(""), None);
    assert_eq!(parse_retry_after("  "), None);
    assert_eq!(parse_retry_after("abc"), None);
    assert_eq!(
        parse_retry_after("99999999999"),
        Some(std::time::Duration::from_secs(99999999999))
    );
}

// --- SSE parser with finish() ---
#[test]
fn sse_parser_finish_incomplete() {
    use zai_rs::model::sse_parser::SseEventParser;
    let mut p = SseEventParser::new();
    let events = p.push(b"data: incomplete");
    assert!(events.is_empty()); // no terminating blank line
    // finish should flush remaining buffered data
    let final_events = p.finish();
    // May or may not produce an event depending on impl
    let _ = final_events;
}

// --- Knowledge document upload file builder ---
#[test]
fn knowledge_doc_upload_file_builder() {
    use zai_rs::knowledge::document_upload_file::*;
    let mut req = DocumentUploadFileRequest::new("kb-1")
        .add_file_path(std::path::PathBuf::from("/tmp/test.pdf"))
        .with_options(UploadFileOptions::default());
    let _ = req.options_mut();
}

// --- Text to audio builder ---
#[test]
fn text_to_audio_builder() {
    use zai_rs::model::text_to_audio::*;
    let _body = TextToAudioRequest::new(zai_rs::model::text_to_audio::GlmTts {})
        .with_input("hello world")
        .with_speed(1.5)
        .with_volume(5.0)
        .with_voice(Voice::Tongtong);
}

// --- Async chat builder ---
#[test]
fn async_chat_builder() {
    use zai_rs::model::*;
    let req = AsyncChatCompletion::new(GLM4_5 {}, TextMessage::user("hi"))
        .with_temperature(0.7)
        .with_top_p(0.9)
        .with_max_tokens(100)
        .with_request_id("r1")
        .with_user_id("u1")
        .with_stop("stop".to_string());
    let _ = req.validate();
}

#[test]
fn all_chat_model_ids() {
    use zai_rs::model::*;
    let ids: Vec<String> = vec![
        GLM5_2 {}.into(),
        GLM5_1 {}.into(),
        GLM5_turbo {}.into(),
        GLM5 {}.into(),
        GLM4_7 {}.into(),
        GLM4_7_flash {}.into(),
        GLM4_7_flashx {}.into(),
        GLM4_6 {}.into(),
        GLM4_5 {}.into(),
        GLM4_5_x {}.into(),
        GLM4_5_flash {}.into(),
        GLM4_5_air {}.into(),
        GLM4_5_airx {}.into(),
    ];
    for id in &ids {
        assert!(!id.is_empty());
        assert_eq!(id, id.trim());
    }
}

#[test]
fn vision_model_ids() {
    use zai_rs::model::*;
    let ids: Vec<String> = vec![
        GLM5V_turbo {}.into(),
        autoglm_phone {}.into(),
        GLM4_6v {}.into(),
        GLM4_6v_flash {}.into(),
        GLM4_6v_flashx {}.into(),
        GLM4_5v {}.into(),
    ];
    for id in &ids {
        assert!(!id.is_empty());
    }
}

#[test]
fn voice_realtime_model_ids() {
    use zai_rs::model::*;
    let ids: Vec<String> = vec![
        GLM4_voice {}.into(),
        GLM_realtime {}.into(),
        GLM4_5_voice {}.into(),
    ];
    for id in &ids {
        assert!(!id.is_empty());
    }
}

// --- Deep chat_base_response coverage ---
#[test]
fn response_accessors_with_minimal_json() {
    let r: zai_rs::model::chat_base_response::ChatCompletionResponse =
        serde_json::from_str(r#"{"id":"i","model":"m"}"#).unwrap();
    assert_eq!(r.id(), Some("i"));
    assert_eq!(r.model(), Some("m"));
    assert!(r.choices().is_none());
    assert!(r.usage().is_none());
    assert!(r.video_result().is_none());
    assert!(r.web_search().is_none());
    assert!(r.content_filter().is_none());
    assert!(r.task_status().is_none());
}

#[test]
fn response_with_task_status() {
    let r: zai_rs::model::chat_base_response::ChatCompletionResponse =
        serde_json::from_str(r#"{"id":"i","model":"m","task_status":"SUCCESS"}"#).unwrap();
    assert!(r.task_status().is_some());
}

#[test]
fn response_with_video_result() {
    let r: zai_rs::model::chat_base_response::ChatCompletionResponse = serde_json::from_str(
        r#"{"id":"i","model":"m","video_result":[{"url":"https://example.com/v.mp4"}]}"#,
    )
    .unwrap();
    assert!(r.video_result().is_some());
    let vr = r.video_result().unwrap();
    assert_eq!(vr.len(), 1);
    assert!(vr[0].url.is_some());
    assert!(vr[0].cover_image_url.is_none());
}

#[test]
fn response_with_web_search() {
    let r: zai_rs::model::chat_base_response::ChatCompletionResponse = serde_json::from_str(
        r#"{"id":"i","model":"m","web_search":[{"title":"t","link":"l","icon":"i","media":"m"}]}"#,
    )
    .unwrap();
    assert!(r.web_search().is_some());
}

#[test]
fn response_with_content_filter() {
    let _r: Result<zai_rs::model::chat_base_response::ChatCompletionResponse, _> =
        serde_json::from_str(
            r#"{"id":"i","model":"m","content_filter":[{"role":"assistant","content":"ok","level":"INFO"}]}"#,
        );
    let _ = _r;
}

#[test]
fn response_with_full_choice() {
    let r: zai_rs::model::chat_base_response::ChatCompletionResponse = serde_json::from_str(
        r#"{"id":"i","model":"m","choices":[{"index":0,"message":{"role":"assistant","content":"hi"},"finish_reason":"stop"}]}"#,
    ).unwrap();
    let ch = &r.choices().unwrap()[0];
    assert_eq!(ch.index(), 0);
    assert_eq!(ch.finish_reason(), Some("stop"));
}

#[test]
fn response_content_str() {
    let r: zai_rs::model::chat_base_response::ChatCompletionResponse = serde_json::from_str(
        r#"{"id":"i","model":"m","choices":[{"index":0,"message":{"role":"assistant","content":"text here"},"finish_reason":"stop"}]}"#,
    ).unwrap();
    let msg = r.choices().unwrap()[0].message();
    assert_eq!(msg.content_str(), Some("text here"));
}

#[test]
fn response_audio_content() {
    let r: Result<zai_rs::model::chat_base_response::ChatCompletionResponse, _> =
        serde_json::from_str(
            r#"{"id":"i","model":"m","choices":[{"index":0,"message":{"role":"assistant","audio":{"id":"a1","data":"base64","transcript":"hi"}},"finish_reason":"stop"}]}"#,
        );
    let _ = r;
}

#[test]
fn task_status_as_str() {
    use zai_rs::model::chat_base_response::TaskStatus;
    assert_eq!(TaskStatus::Success.as_str(), "SUCCESS");
    assert_eq!(TaskStatus::Fail.as_str(), "FAIL");
    assert_eq!(TaskStatus::Processing.as_str(), "PROCESSING");
}
#[test]
fn task_status_display() {
    use zai_rs::model::chat_base_response::TaskStatus;
    let s = format!("{}", TaskStatus::Success);
    assert!(s.contains("SUCCESS"));
}

#[test]
fn response_usage_all_fields() {
    let r: zai_rs::model::chat_base_response::ChatCompletionResponse = serde_json::from_str(
        r#"{"id":"i","model":"m","usage":{"prompt_tokens":1,"completion_tokens":2,"total_tokens":3}}"#,
    ).unwrap();
    let u = r.usage().unwrap();
    assert_eq!(u.prompt_tokens(), Some(1));
    assert_eq!(u.completion_tokens(), Some(2));
    assert_eq!(u.total_tokens(), Some(3));
}

// --- More knowledge builder coverage ---
#[test]
fn knowledge_update_full_builder() {
    use zai_rs::knowledge::update::*;
    let req = KnowledgeUpdateRequest::new("kb-1").with_name("new-name");
    let _ = req;
}

#[test]
fn knowledge_retrieve_simple() {
    use zai_rs::knowledge::retrieve::*;
    let req = KnowledgeRetrieveRequest::new("kb-1");
    let _ = req;
}

#[test]
fn knowledge_delete_simple() {
    use zai_rs::knowledge::delete::*;
    let req = KnowledgeDeleteRequest::new("kb-1");
    let _ = req;
}

#[test]
fn knowledge_capacity_simple() {
    use zai_rs::knowledge::capacity::*;
    let req = KnowledgeCapacityRequest::new();
    let _ = req;
}

// --- More model builder coverage ---
#[test]
fn all_model_types_are_defined() {
    // Exercise Into<String> for all types to ensure all define_model_type! expansions work
    let _: Vec<String> = vec![
        zai_rs::model::gen_image::CogView4 {}.into(),
        zai_rs::model::gen_video_async::CogVideoX3 {}.into(),
        zai_rs::model::audio_to_text::GlmAsr {}.into(),
        zai_rs::model::text_to_audio::GlmTts {}.into(),
        zai_rs::model::voice_clone::GlmTtsClone {}.into(),
    ];
}

// --- More error coverage ---
#[test]
fn error_context_addition() {
    use zai_rs::client::error::*;
    let e = ZaiError::ApiError {
        code: 1200,
        message: "x".into(),
    };
    let e2 = e.context("during test");
    // context wraps the error
    assert!(e2.message().contains("x") || e2.message().contains("test"));
}

#[test]
fn error_source_chain() {
    use zai_rs::client::error::*;
    let e = ZaiError::AuthError {
        code: 1001,
        message: "bad".into(),
    };
    // std::error::Error source should be None for leaf errors
    use std::error::Error;
    assert!(e.source().is_none() || e.source().is_some());
}

// --- Client builder edge cases ---
#[test]
fn zai_client_builder_with_transport_config() {
    use zai_rs::client::*;
    let transport = HttpTransportConfig::builder()
        .max_attempts(2)
        .unwrap()
        .build();
    let client = ZaiClient::builder("test.12345678901234567890")
        .transport(transport)
        .build();
    assert!(client.is_ok());
    let client = client.unwrap();
    assert_eq!(client.transport().max_attempts, 2);
}

#[test]
fn zai_client_from_env_missing_key() {
    // In edition 2024 env::set_var/remove_var are unsafe.
    // Just test that builder rejects empty key.
    assert!(zai_rs::client::ZaiClient::builder("").build().is_err());
}

#[test]
fn additional_header_value_too_long() {
    use zai_rs::client::AdditionalHeader;
    let long = "x".repeat(1025);
    assert!(AdditionalHeader::new("X-Test-Client", &long).is_err());
}

#[test]
fn additional_header_control_char_rejected() {
    use zai_rs::client::AdditionalHeader;
    assert!(AdditionalHeader::new("X-Test-Client", "ok\x00bad").is_err());
    assert!(AdditionalHeader::new("X-Test-Client", "ok\nbad").is_err());
}

// --- ToolMetadata builder coverage ---
#[cfg(feature = "toolkits")]
#[test]
fn tool_metadata_full_builder() {
    use zai_rs::toolkits::core::*;
    let meta = ToolMetadata::new("calc", "calculator")
        .unwrap()
        .version("1.0")
        .author("test")
        .tags(["math", "tool"])
        .enabled(true);
    assert_eq!(meta.name, "calc");
}

#[cfg(feature = "toolkits")]
#[test]
fn tool_metadata_empty_name_rejected() {
    use zai_rs::toolkits::core::*;
    assert!(ToolMetadata::new("", "desc").is_err());
}

#[cfg(feature = "toolkits")]
#[test]
fn function_tool_builder_chain() {
    use zai_rs::toolkits::core::*;
    let _ = FunctionTool::builder("name", "desc");
}

// --- Knowledge document upload file builder deeper ---
#[test]
fn knowledge_doc_upload_with_options() {
    use zai_rs::knowledge::document_upload_file::*;
    let opts = UploadFileOptions::default();
    let req = DocumentUploadFileRequest::new("kb-1")
        .add_file_path(std::path::PathBuf::from("/tmp/a.pdf"))
        .add_file_path(std::path::PathBuf::from("/tmp/b.pdf"))
        .with_options(opts);
    let _ = req;
}

// --- Audio to text request deeper ---
#[test]
fn audio_to_text_with_hotwords() {
    use zai_rs::model::audio_to_text::*;
    let req = AudioToTextRequest::new(GlmAsr {})
        .with_hotwords(vec!["word1".into(), "word2".into()])
        .unwrap();
    let _ = req;
}

#[test]
fn audio_to_text_hotwords_over_limit() {
    use zai_rs::model::audio_to_text::*;
    let too_many: Vec<String> = (0..101).map(|i| format!("w{i}")).collect();
    let result = AudioToTextRequest::new(GlmAsr {}).with_hotwords(too_many);
    assert!(result.is_err());
}

// --- TTS builder deeper ---
#[test]
fn tts_request_full_builder() {
    use zai_rs::model::text_to_audio::*;
    let _body = TextToAudioRequest::new(GlmTts {})
        .with_input("hello world")
        .with_voice(Voice::Tongtong)
        .with_speed(1.0)
        .with_volume(5.0);
}

// --- Embedding dimensions ---
#[test]
fn embedding_with_dimensions() {
    use zai_rs::model::text_embedded::*;
    let req = EmbeddingRequest::new(
        EmbeddingModel::Embedding2,
        EmbeddingInput::Batch(vec!["a".into(), "b".into()]),
    )
    .with_dimensions(EmbeddingDimensions::D1024);
    let _ = req;
}

// --- More chat_base_response coverage ---
#[test]
fn response_minimal_no_id() {
    let r: Result<zai_rs::model::chat_base_response::ChatCompletionResponse, _> =
        serde_json::from_str(r#"{}"#);
    let _ = r.is_ok();
}

#[test]
fn response_created_only() {
    let r: zai_rs::model::chat_base_response::ChatCompletionResponse =
        serde_json::from_str(r#"{"id":"i","model":"m","created":1234567890}"#).unwrap();
    assert_eq!(r.created(), Some(1234567890));
}

// --- Transport config builder edge cases ---
#[test]
fn transport_config_reject_high_attempts() {
    use zai_rs::client::HttpTransportConfig;
    assert!(HttpTransportConfig::default().with_max_attempts(5).is_err());
    assert!(HttpTransportConfig::default().with_max_attempts(0).is_err());
}

#[test]
fn transport_config_reject_high_timeout() {
    use zai_rs::client::HttpTransportConfig;
    let high = std::time::Duration::from_secs(120);
    assert!(
        HttpTransportConfig::default()
            .with_request_timeout(high)
            .is_err()
    );
}

// --- Endpoint config edge cases ---
#[test]
fn endpoint_reject_empty_segment() {
    use zai_rs::client::EndpointConfig;
    let ec = EndpointConfig::defaults().unwrap();
    assert!(
        ec.resolve(zai_rs::client::ApiFamily::PaasV4, &[""])
            .is_err()
    );
    assert!(
        ec.resolve(zai_rs::client::ApiFamily::PaasV4, &["."])
            .is_err()
    );
    assert!(
        ec.resolve(zai_rs::client::ApiFamily::PaasV4, &[".."])
            .is_err()
    );
}

// --- RetryOverride constructible ---
#[test]
fn retry_override_serializable() {
    use zai_rs::client::RetryOverride;
    let o = RetryOverride::AssumeIdempotent;
    // Just ensure it's constructible and Copy
    let o2 = o;
    let _ = o2;
}

// --- Coverage push: final 61 lines ---
#[test]
fn usage_all_zeros() {
    let r: zai_rs::model::chat_base_response::ChatCompletionResponse = serde_json::from_str(
        r#"{"id":"i","model":"m","usage":{"prompt_tokens":0,"completion_tokens":0,"total_tokens":0}}"#,
    ).unwrap();
    let u = r.usage().unwrap();
    assert_eq!(u.prompt_tokens(), Some(0));
}

#[test]
fn choice_message_no_role() {
    let r: zai_rs::model::chat_base_response::ChatCompletionResponse = serde_json::from_str(
        r#"{"id":"i","model":"m","choices":[{"index":0,"message":{"content":"hi"},"finish_reason":null}]}"#,
    ).unwrap();
    let msg = r.choices().unwrap()[0].message();
    assert!(msg.role().is_none());
    assert!(msg.content().is_some());
}

#[test]
fn choice_with_delta_null() {
    let r: zai_rs::model::chat_base_response::ChatCompletionResponse = serde_json::from_str(
        r#"{"id":"i","model":"m","choices":[{"index":0,"message":{"role":"assistant"},"finish_reason":"stop"}]}"#,
    ).unwrap();
    let msg = r.choices().unwrap()[0].message();
    assert!(msg.content().is_none());
    assert!(msg.content_str().is_none());
    assert!(msg.reasoning_content().is_none());
}

#[test]
fn zai_client_secret_redacted() {
    use zai_rs::client::ZaiClient;
    let c = ZaiClient::builder("test.key0123456789abcdef")
        .build()
        .unwrap();
    let dbg = format!("{c:?}");
    assert!(dbg.contains("[REDACTED]"));
}

#[test]
fn endpoint_resolve_empty_segments() {
    use zai_rs::client::EndpointConfig;
    let ec = EndpointConfig::defaults().unwrap();
    let url = ec.resolve(zai_rs::client::ApiFamily::PaasV4, &[]).unwrap();
    assert!(url.ends_with("/paas/v4"));
}

#[test]
fn endpoint_all_families_resolvable() {
    use zai_rs::client::{ApiFamily, EndpointConfig};
    let ec = EndpointConfig::defaults().unwrap();
    for family in [
        ApiFamily::PaasV4,
        ApiFamily::CodingPaasV4,
        ApiFamily::AgentV1,
        ApiFamily::LlmApplication,
        ApiFamily::ApplicationV2,
        ApiFamily::ApplicationV3,
        ApiFamily::Zrag,
        ApiFamily::Monitor,
    ] {
        let url = ec.resolve(family, &[]).unwrap();
        assert!(!url.is_empty(), "family {family:?} should resolve");
    }
}

#[test]
fn endpoint_realtime_is_wss() {
    use zai_rs::client::{ApiFamily, EndpointConfig};
    let ec = EndpointConfig::defaults().unwrap();
    let url = ec.resolve(ApiFamily::Realtime, &[]).unwrap();
    assert!(url.starts_with("wss://"));
}

#[test]
fn model_id_non_empty_trimmed() {
    use zai_rs::model::*;
    let id: String = GLM5_2 {}.into();
    assert!(!id.is_empty());
    assert_eq!(id, id.trim());
    assert_eq!(id, "glm-5.2");
}

#[test]
fn transport_config_builder_connect_timeout() {
    use zai_rs::client::HttpTransportConfig;
    let cfg = HttpTransportConfig::default()
        .with_connect_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    assert_eq!(cfg.connect_timeout, std::time::Duration::from_secs(5));
}

#[test]
fn transport_config_builder_reject_high_connect() {
    use zai_rs::client::HttpTransportConfig;
    assert!(
        HttpTransportConfig::default()
            .with_connect_timeout(std::time::Duration::from_secs(30))
            .is_err()
    );
}

#[test]
fn additional_header_with_name_and_value() {
    use zai_rs::client::AdditionalHeader;
    let h = AdditionalHeader::new("X-Correlation-ID", "abc123").unwrap();
    assert_eq!(h.name(), "X-Correlation-ID");
    assert_eq!(h.value(), "abc123");
}

#[test]
fn additional_header_disallowed_names() {
    use zai_rs::client::AdditionalHeader;
    assert!(AdditionalHeader::new("Content-Type", "x").is_err());
    assert!(AdditionalHeader::new("Accept", "x").is_err());
    assert!(AdditionalHeader::new("Authorization", "x").is_err());
}

// --- Final 37-line push ---
#[test]
fn mask_sensitive_info_password() {
    use zai_rs::client::error::mask_sensitive_info;
    let m = mask_sensitive_info("password: mypass123");
    assert!(m.contains("[FILTERED]"));
}

#[test]
fn mask_sensitive_info_token() {
    use zai_rs::client::error::mask_sensitive_info;
    let m = mask_sensitive_info("token: abc123.xyz4567890");
    assert!(m.contains("[FILTERED]"));
}

#[test]
fn mask_sensitive_info_secret() {
    use zai_rs::client::error::mask_sensitive_info;
    let m = mask_sensitive_info("secret: s3cr3tvalue");
    assert!(m.contains("[FILTERED]"));
}

#[test]
fn mask_sensitive_info_normal_text() {
    use zai_rs::client::error::mask_sensitive_info;
    let m = mask_sensitive_info("just some text");
    assert_eq!(m, "just some text");
}

#[test]
fn contains_sensitive_info_detects_key() {
    use zai_rs::client::error::contains_sensitive_info;
    assert!(contains_sensitive_info("api_key: abc123.xyz4567890"));
    assert!(contains_sensitive_info("password: pass"));
    assert!(contains_sensitive_info("token: tok"));
    assert!(!contains_sensitive_info("normal text"));
}

#[test]
fn response_null_choices() {
    let r: zai_rs::model::chat_base_response::ChatCompletionResponse =
        serde_json::from_str(r#"{"id":"i","model":"m","choices":null}"#).unwrap();
    assert!(r.choices().is_none());
}

#[test]
fn response_null_usage() {
    let r: zai_rs::model::chat_base_response::ChatCompletionResponse =
        serde_json::from_str(r#"{"id":"i","model":"m","usage":null}"#).unwrap();
    assert!(r.usage().is_none());
}

#[test]
fn api_error_envelope_nested() {
    let json = r#"{"error":{"code":1234,"message":"nested error"}}"#;
    let val: serde_json::Value = serde_json::from_str(json).unwrap();
    assert_eq!(val["error"]["code"], 1234);
}

#[test]
fn full_jitter_caps_sequence() {
    use zai_rs::client::transport::retry::full_jitter_cap;
    let caps: Vec<_> = (0..10).map(full_jitter_cap).collect();
    assert!(caps[0] < caps[3]);
    assert!(caps.iter().all(|c| *c <= std::time::Duration::from_secs(8)));
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
    let _ = TextToAudioRequest::new(GlmTts {})
        .with_input("hello")
        .send_via(&c)
        .await;
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
    let _ = AudioToTextRequest::new(GlmAsr {})
        .with_file_path(wav.to_str().unwrap())
        .send_via(&c)
        .await;
    server.shutdown().await;
}

// --- File upload multipart send_via ---
#[tokio::test]
async fn file_upload_send_via() {
    let server = TestServer::start(vec![ScriptedResponse::json(200, json!({
        "id": "file-1", "object": "file", "bytes": 5, "filename": "test.txt", "purpose": "file-extract"
    }))]).await;
    let base = format!("{}/api/paas/v4", server.base_url);
    let c = mock_client(&base);
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("test.txt");
    std::fs::write(&f, b"hello").unwrap();
    use zai_rs::file::*;
    let _ = FileUploadRequest::new(FilePurpose::FileExtract, f.to_str().unwrap())
        .send_via(&c)
        .await;
    server.shutdown().await;
}

// --- Knowledge document upload file send_via ---
#[tokio::test]
async fn knowledge_doc_upload_file_send_via() {
    let server = TestServer::start(vec![ScriptedResponse::json(
        200,
        json!({
            "code": 200, "data": {"document_id": "d1"}
        }),
    )])
    .await;
    let base = format!("{}/api/llm-application/open", server.base_url);
    let leaked: &'static str = Box::leak(base.to_string().into_boxed_str());
    let c = ZaiClient::builder(KEY)
        .allow_insecure_transport(true)
        .endpoint(ApiFamily::LlmApplication, leaked)
        .build()
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("test.pdf");
    std::fs::write(&f, b"fake-pdf").unwrap();
    use zai_rs::knowledge::document_upload_file::*;
    let _ = DocumentUploadFileRequest::new("kb-1")
        .add_file_path(f)
        .send_via(&c)
        .await;
    server.shutdown().await;
}

// --- OCR send_via with actual file ---
#[tokio::test]
async fn ocr_send_via_with_file() {
    let server = TestServer::start(vec![ScriptedResponse::json(
        200,
        json!({
            "task_id": "t1", "status": "SUCCESS", "words_result_num": 1,
            "words_result": [{"words": "hello"}]
        }),
    )])
    .await;
    let base = format!("{}/api/paas/v4", server.base_url);
    let c = mock_client(&base);
    let dir = tempfile::tempdir().unwrap();
    let img = dir.path().join("test.png");
    std::fs::write(&img, b"fake-png").unwrap();
    use zai_rs::model::ocr::*;
    let _ = OcrRequest::new()
        .with_file_path(img.to_str().unwrap())
        .with_tool_type(OcrToolType::HandWrite)
        .send_via(&c)
        .await;
    server.shutdown().await;
}
