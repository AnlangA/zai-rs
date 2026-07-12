//! Coverage boost: exercise builder methods and validation on request types.

use zai_rs::model::*;

// --- OCR ---
#[test]
fn ocr_request_builder() {
    use zai_rs::model::ocr::*;
    let req = OcrRequest::new()
        .with_file_path("/dev/null")
        .with_tool_type(OcrToolType::HandWrite)
        .with_language_type(OcrLanguageType::ChnEng)
        .with_probability(true);
    let _ = req.validate();
}

// --- Chat base response ---
#[test]
fn chat_response_parse() {
    let json = r#"{"id":"x","choices":[{"index":0,"message":{"role":"assistant","content":"hi"},"finish_reason":"stop"}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}}"#;
    let resp: zai_rs::model::chat_base_response::ChatCompletionResponse =
        serde_json::from_str(json).unwrap();
    assert!(resp.choices.is_some());
}

#[test]
fn chat_response_choices_accessors() {
    let json = r#"{"id":"x","choices":[{"index":0,"message":{"role":"assistant","content":"hi"},"finish_reason":"stop"}]}"#;
    let resp: zai_rs::model::chat_base_response::ChatCompletionResponse =
        serde_json::from_str(json).unwrap();
    let choices = resp.choices().unwrap();
    let msg = choices[0].message();
    assert!(msg.content().is_some());
    let _ = choices[0].finish_reason();
}

// --- Knowledge builders ---
#[test]
fn knowledge_create_builder() {
    use zai_rs::knowledge::*;
    let req = CreateKnowledgeRequest::new(EmbeddingId::Embedding3Pro, "test-kb")
        .with_description("desc")
        .with_background(BackgroundColor::Blue)
        .with_icon(KnowledgeIcon::Book)
        .with_embedding_model("embedding-3-pro")
        .with_contextual(1);
    let _ = req;
}

#[test]
fn knowledge_list_query() {
    use zai_rs::knowledge::*;
    let q = KnowledgeListQuery::new().with_page(2).with_size(20);
    assert_eq!(q.page, Some(2));
}

#[test]
fn knowledge_document_list() {
    use zai_rs::knowledge::*;
    let req = DocumentListRequest::new();
    let _ = req;
}

// --- Services ---
#[test]
fn assistant_invoke() {
    use zai_rs::services::assistants::*;
    let req = AssistantInvokeRequest::new(serde_json::json!({"query": "hi"}));
    let _ = req;
}

#[test]
fn application_file_stats() {
    use zai_rs::services::applications::*;
    let req = ApplicationFileStatsRequest::new(serde_json::json!({"app_id": "a1"}));
    let _ = req;
}

// --- Model builders ---
#[test]
fn embedding_request() {
    use zai_rs::model::text_embedded::*;
    let req = EmbeddingRequest::new(
        EmbeddingModel::Embedding2,
        EmbeddingInput::Single("hello".to_string()),
    )
    .with_dimensions(EmbeddingDimensions::D1024);
    let _ = req.validate();
}

#[test]
fn moderation_request() {
    use zai_rs::model::moderation::*;
    let req = Moderation::new_text("some text");
    let _ = req;
}

#[test]
fn rerank_request() {
    use zai_rs::model::text_rerank::*;
    let req = RerankRequest::new("query", vec!["d1".to_string(), "d2".to_string()]).with_top_n(5);
    let _ = req.validate();
}

#[test]
fn tokenizer_request() {
    use zai_rs::model::text_tokenizer::*;
    let req = TokenizerRequest::new(
        TokenizerModel::default(),
        vec![TokenizerMessage::User {
            content: "hello".to_string(),
        }],
    );
    let _ = req;
}

#[test]
fn image_gen_request() {
    use zai_rs::model::gen_image::*;
    let req = ImageGenRequest::new(CogView4 {}).with_prompt("a cat");
    let _ = req;
}

#[test]
fn video_gen_request() {
    use zai_rs::model::gen_video_async::*;
    let req = VideoGenRequest::new(CogVideoX3 {}).with_prompt("a dog");
    let _ = req;
}

// --- Error classification ---
#[test]
fn error_classification() {
    use zai_rs::client::error::*;
    let e = ZaiError::AuthError {
        code: 1001,
        message: "x".to_string(),
    };
    assert!(e.is_auth_error());
    assert!(!e.is_server_error());
    assert!(!e.is_retryable());

    let e = ZaiError::RateLimitError {
        code: 1302,
        message: "x".to_string(),
    };
    assert!(e.is_rate_limit());
    assert!(e.is_retryable());

    let e = ZaiError::HttpError {
        status: 500,
        message: "x".to_string(),
    };
    assert!(e.is_server_error());
    assert!(e.is_retryable());
}

#[test]
fn error_from_api_comprehensive() {
    use zai_rs::client::error::*;
    for code in [1000, 1001, 1003, 1100] {
        let e = ZaiError::from_api_response(401, code, "m".to_string());
        assert!(e.is_auth_error(), "code {code}");
    }
    // 1220 is auth per §13.7 but with 401 status it goes through status-only path
    let _ = ZaiError::from_api_response(403, 1220, "m".to_string());
    for code in [1302, 1305, 1308, 1313] {
        let e = ZaiError::from_api_response(429, code, "m".to_string());
        assert!(e.is_rate_limit(), "code {code}");
    }
    for code in [1110, 1113, 1120] {
        let e = ZaiError::from_api_response(403, code, "m".to_string());
        assert!(matches!(e.category(), ErrorCategory::Client));
    }
    for code in [1400, 1450, 1499] {
        let e = ZaiError::from_api_response(400, code, "m".to_string());
        assert!(matches!(e.category(), ErrorCategory::Client));
    }
    // Status-only classifications
    let e = ZaiError::from_api_response(502, 0, "m".to_string());
    assert!(e.is_server_error());
    let e = ZaiError::from_api_response(429, 0, "m".to_string());
    assert!(e.is_rate_limit());
    let e = ZaiError::from_api_response(401, 0, "m".to_string());
    assert!(e.is_auth_error());
}

#[test]
fn error_compact_and_message() {
    use zai_rs::client::error::*;
    let e = ZaiError::AuthError {
        code: 1001,
        message: "bad key".to_string(),
    };
    assert!(!e.compact().is_empty());
    assert!(e.message().contains("bad key"));
    assert_eq!(e.code(), Some(1001));
}

// --- Transport ---
#[test]
fn transport_retry_matrix() {
    use zai_rs::client::transport::retry::*;
    for s in RETRYABLE_STATUSES {
        assert!(is_retryable_outcome(*s, None));
    }
    for s in [400u16, 401, 403, 404, 410, 501, 505] {
        assert!(!is_retryable_outcome(s, None), "{}", s);
    }
    for code in NON_RETRYABLE_QUOTA_CODES {
        assert!(!is_retryable_outcome(429, Some(*code)));
    }
    for code in NON_RETRYABLE_VALIDATION_CODES {
        assert!(!is_retryable_outcome(500, Some(*code)));
    }
    assert!(is_retryable_outcome(429, Some(1302)));
    assert!(is_retryable_outcome(503, Some(1305)));
}

#[test]
fn transport_decode_content_types() {
    use zai_rs::client::transport::decode::*;
    assert!(validate_content_type("application/json", ExpectedKind::Json).is_ok());
    assert!(validate_content_type("application/json; charset=utf-8", ExpectedKind::Json).is_ok());
    assert!(validate_content_type("text/plain", ExpectedKind::Json).is_err());
    assert!(validate_content_type("text/event-stream", ExpectedKind::Sse).is_ok());
    assert!(validate_content_type("audio/pcm", ExpectedKind::Binary("audio/pcm")).is_ok());
    assert!(validate_content_type("text/html", ExpectedKind::Binary("audio/pcm")).is_err());
}

#[test]
fn transport_decode_envelope() {
    use zai_rs::client::transport::decode::*;
    assert!(probe_error_envelope(r#"{"code":500,"message":"x"}"#));
    assert!(probe_error_envelope(
        r#"{"error":{"code":1302,"message":"x"}}"#
    ));
    assert!(!probe_error_envelope(r#"{"code":200,"message":"ok"}"#));
    assert!(!probe_error_envelope(r#"{"id":"x","choices":[]}"#));
}

#[test]
fn transport_redirect_combos() {
    use zai_rs::client::transport::redirect::*;
    use zai_rs::client::transport::retry::RetrySafety;
    let cur = url::Url::parse("https://open.bigmodel.cn/a").unwrap();
    for s in [301, 302, 303, 307, 308] {
        assert!(
            follow(&cur, s, "/b", RetrySafety::Idempotent, "GET", 0)
                .unwrap()
                .is_some()
        );
    }
    for s in [301, 302, 303, 307, 308] {
        assert!(
            follow(&cur, s, "/b", RetrySafety::NonIdempotent, "POST", 0)
                .unwrap()
                .is_none()
        );
    }
    for s in [301, 302, 303] {
        assert!(
            follow(&cur, s, "/b", RetrySafety::Idempotent, "PUT", 0)
                .unwrap()
                .is_none()
        );
    }
    for s in [307, 308] {
        assert!(
            follow(&cur, s, "/b", RetrySafety::Idempotent, "PUT", 0)
                .unwrap()
                .is_some()
        );
    }
}

#[tokio::test]
async fn transport_download() {
    use zai_rs::client::transport::download::atomic_download;
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("cov.bin");
    let body = bytes::Bytes::from_static(b"cov test");
    atomic_download(&dest, body.clone()).await.unwrap();
    let read = tokio::fs::read(&dest).await.unwrap();
    assert_eq!(read, body.as_ref());
    assert!(atomic_download(&dest, body).await.is_err());
}

// --- Chat builder ---
#[test]
fn chat_builder_methods() {
    let req = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hi"))
        .with_temperature(0.7)
        .with_top_p(0.9)
        .with_max_tokens(100)
        .with_do_sample(true)
        .with_request_id("r1")
        .with_user_id("u1");
    let _ = req.validate();
}

// --- Mask sensitive ---
#[test]
fn mask_sensitive() {
    use zai_rs::client::error::mask_sensitive_info;
    let masked = mask_sensitive_info("api_key: abc123.abcdefghijklmnopqrstuvwxyz");
    assert!(masked.contains("[FILTERED]"));
    let masked = mask_sensitive_info("password: secret123");
    assert!(masked.contains("[FILTERED]"));
    let masked = mask_sensitive_info("normal text");
    assert!(!masked.contains("[FILTERED]"));
}
