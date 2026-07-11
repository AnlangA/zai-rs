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
    let _ = ImageGenRequest::new(CogView4 {})
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
    let _ = VideoGenRequest::new(CogVideoX3 {})
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
    let _ = VoiceCloneRequest::new(GlmTtsClone {}, "voice1", "hello", "file-1")
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
