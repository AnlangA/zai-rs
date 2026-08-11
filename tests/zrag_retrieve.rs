//! External-crate contract tests for the production ZRAG retrieval route.

mod support;

use serde_json::json;
use support::http_server::{ScriptedResponse, TestServer};
use zai_rs::{
    ZaiClient,
    client::ApiFamily,
    zrag::{
        ZragFilterValueType, ZragImagePart, ZragIndexTypeFilter, ZragKnowledge, ZragQaIntervention,
        ZragRecallMethod, ZragRetrieveMessage, ZragRetrieveRequest, ZragSearchFilters,
        ZragTagFilter, ZragTagFilterOperator,
    },
};

const KEY: &str = "test.12345678901234567890";

fn client_for(server: &TestServer) -> ZaiClient {
    ZaiClient::builder(KEY)
        .allow_insecure_transport(true)
        .endpoint(ApiFamily::Zrag, format!("{}/api/zrag", server.base_url))
        .build()
        .unwrap()
}

#[tokio::test]
async fn public_request_dispatches_the_zrag_route_and_decodes_typed_data() {
    let server = TestServer::start(vec![ScriptedResponse::json(
        200,
        json!({
            "code": 200,
            "message": "ok",
            "data": {
                "contents": [{
                    "id": "slice-1",
                    "know_id": "knowledge-1",
                    "doc_id": "document-1",
                    "text": "answer",
                    "score": 0.9,
                    "metadata": {
                        "doc_type": "pdf",
                        "doc_name": "guide.pdf",
                        "page_index": 2
                    }
                }],
                "rewritten_query": {
                    "original_query": "question",
                    "multi_queries": ["rewritten question"]
                },
                "elapsed_ms": 12,
                "total_tokens": 34,
                "request_id": "request-1"
            },
            "future_field": {"accepted": true}
        }),
    )])
    .await;
    let client = client_for(&server);

    let response = ZragRetrieveRequest::new(vec![
        ZragKnowledge::new("knowledge-1").with_document_ids(vec!["document-1".to_owned()]),
    ])
    .with_multimodal(true)
    .with_query("question")
    .with_image_parts(vec![ZragImagePart::new("https://example.test/image.png")])
    .with_top_k(8)
    .with_top_n(10)
    .with_recall_method(ZragRecallMethod::Mixed)
    .with_recall_ratio(0.8)
    .with_reranking(true)
    .with_rewrite(true)
    .with_expansion(true)
    .with_similarity_threshold(0.2)
    .with_messages(vec![ZragRetrieveMessage::user("previous question")])
    .with_search_filters(
        ZragSearchFilters::new()
            .with_index_types(vec![ZragIndexTypeFilter::new("knowledge-1", 7)])
            .with_tags(vec![ZragTagFilter::new(
                "tag-1",
                ZragFilterValueType::Fixed,
                ZragTagFilterOperator::Contains,
                "filter-value",
                vec!["choice-1".to_owned()],
            )])
            .with_qa_intervention(ZragQaIntervention::new(0.6, vec!["knowledge-1".to_owned()])),
    )
    .send_via(&client)
    .await
    .unwrap();

    assert_eq!(response.code(), Some(200));
    let data = response.data().unwrap();
    assert_eq!(data.request_id(), Some("request-1"));
    assert_eq!(data.contents().unwrap()[0].text(), Some("answer"));
    assert_eq!(
        data.contents().unwrap()[0]
            .metadata()
            .unwrap()
            .document_name(),
        Some("guide.pdf")
    );

    let requests = server.requests();
    assert_eq!(requests.len(), 1);
    let captured = &requests[0];
    assert_eq!(captured.method, "POST");
    assert_eq!(captured.path, "/api/zrag/retrieval/retrieve");
    assert_eq!(
        captured.authorization.as_deref(),
        Some("Bearer test.12345678901234567890")
    );
    assert!(
        captured.headers.iter().any(|(name, value)| {
            name == "content-type" && value.starts_with("application/json")
        })
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&captured.body).unwrap(),
        json!({
            "multimodal": true,
            "knows": [{"id": "knowledge-1", "doc_ids": ["document-1"]}],
            "query": "question",
            "multimodal_parts": [{
                "type": "image_url",
                "url": "https://example.test/image.png"
            }],
            "top_k": 8,
            "top_n": 10,
            "recall_method": "mixed",
            "recall_ratio": 0.8,
            "enable_rerank": true,
            "enable_rewrite": true,
            "enable_expansion": true,
            "similarity_threshold": 0.2,
            "messages": [{"role": "user", "content": "previous question"}],
            "search_filters": {
                "index_types": [{"know_id": "knowledge-1", "index_type_id": 7}],
                "tags": [{
                    "tag_id": "tag-1",
                    "value_type": "fixed",
                    "filter_type": 3,
                    "filter_value": "filter-value",
                    "multiple_value": ["choice-1"]
                }],
                "qa_intervention": {
                    "qa_similarity_threshold": 0.6,
                    "qa_intervention_ids": ["knowledge-1"]
                }
            }
        })
    );

    server.shutdown().await;
}

#[tokio::test]
async fn validation_fails_before_network_io() {
    let server = TestServer::start(vec![ScriptedResponse::json(200, json!({"code": 200}))]).await;
    let client = client_for(&server);

    let error = ZragRetrieveRequest::new(vec![ZragKnowledge::new("knowledge-1")])
        .send_via(&client)
        .await
        .unwrap_err();

    assert_eq!(
        error.code(),
        Some(zai_rs::client::error::codes::SDK_VALIDATION)
    );
    assert!(server.requests().is_empty());
    server.shutdown().await;
}

#[tokio::test]
async fn business_error_is_projected_once_without_retrying_the_post() {
    let server = TestServer::start(vec![ScriptedResponse::json(
        200,
        json!({
            "code": 1302,
            "message": "rate limited",
            "request_id": "request-1"
        }),
    )])
    .await;
    let client = client_for(&server);

    let error = ZragRetrieveRequest::new(vec![ZragKnowledge::new("knowledge-1")])
        .with_query("question")
        .send_via(&client)
        .await
        .unwrap_err();

    assert!(error.is_rate_limit());
    assert_eq!(server.requests().len(), 1, "POST must not be replayed");
    server.shutdown().await;
}
