use std::collections::HashMap;

use serde::Deserialize;

use super::*;

/// Every operation frozen in `spec/contracts/operations.json`.
///
/// Keeping this list explicit makes additions and removals reviewable. Routes
/// for Coding Plan and realtime are intentionally separate because those
/// operations are not part of the frozen HTTP operation contract.
const CONTRACT_ROUTES: &[Route] = &[
    AGENTS_INVOKE,
    AGENTS_ASYNC_RESULT,
    AGENTS_CONVERSATION,
    APPLICATIONS_HISTORY,
    APPLICATIONS_FILE_STATS,
    APPLICATIONS_UPLOAD_FILE,
    APPLICATIONS_SLICE_INFO,
    APPLICATIONS_CREATE_CONVERSATION,
    APPLICATIONS_VARIABLES,
    APPLICATIONS_INVOKE,
    DOCUMENTS_LIST,
    DOCUMENTS_REEMBED,
    DOCUMENTS_IMAGES,
    DOCUMENTS_UPLOAD,
    DOCUMENTS_UPLOAD_URL,
    DOCUMENTS_DELETE,
    DOCUMENTS_GET,
    KNOWLEDGE_LIST,
    KNOWLEDGE_CREATE,
    KNOWLEDGE_CAPACITY,
    KNOWLEDGE_RETRIEVE,
    KNOWLEDGE_DELETE,
    KNOWLEDGE_GET,
    KNOWLEDGE_UPDATE,
    ASSISTANTS_INVOKE,
    ASSISTANTS_CONVERSATIONS,
    ASSISTANTS_LIST,
    TASKS_GET,
    CHAT_COMPLETE_ASYNC,
    IMAGES_GENERATE_ASYNC,
    AUDIO_SYNTHESIZE,
    AUDIO_TRANSCRIBE,
    BATCHES_LIST,
    BATCHES_CREATE,
    BATCHES_GET,
    BATCHES_CANCEL,
    CHAT_COMPLETE,
    EMBEDDINGS_CREATE,
    FILES_LIST,
    FILES_UPLOAD,
    FILES_OCR,
    FILES_PARSE,
    FILES_PARSE_RESULT,
    FILES_PARSE_SYNC,
    FILES_DELETE,
    FILES_GET_CONTENT,
    IMAGES_GENERATE,
    TOOLS_LAYOUT,
    MODERATION_CHECK,
    TOOLS_READER,
    RERANK_CREATE,
    TOKENIZER_COUNT,
    VIDEOS_GENERATE,
    AUDIO_CLONE_VOICE,
    AUDIO_DELETE_VOICE,
    AUDIO_LIST_VOICES,
    TOOLS_WEB_SEARCH,
    ZRAG_CHAT,
    ZRAG_RETRIEVE,
];

#[derive(Debug, Deserialize)]
struct FrozenOperation {
    operation_id: String,
    method: String,
    path: String,
    api_family: String,
}

#[test]
fn registry_exactly_matches_frozen_operations_contract() {
    let contract_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("spec/contracts/operations.json");
    let raw = std::fs::read_to_string(&contract_path)
        .unwrap_or_else(|error| panic!("read {}: {error}", contract_path.display()));
    let frozen: Vec<FrozenOperation> = serde_json::from_str(&raw).unwrap();
    let mut by_id: HashMap<_, _> = frozen
        .iter()
        .map(|operation| (operation.operation_id.as_str(), operation))
        .collect();

    assert_eq!(by_id.len(), frozen.len(), "duplicate frozen operation id");
    assert_eq!(CONTRACT_ROUTES.len(), frozen.len());

    for route in CONTRACT_ROUTES {
        let operation = by_id
            .remove(route.operation_id())
            .unwrap_or_else(|| panic!("route {} is not frozen", route.operation_id()));
        assert_eq!(route.method(), operation.method);
        assert_eq!(format!("{:?}", route.family()), operation.api_family);
        assert_eq!(
            normalized_route_path(*route),
            normalize_parameters(&operation.path)
        );
    }

    assert!(
        by_id.is_empty(),
        "frozen operations missing routes: {by_id:?}"
    );
}

fn normalized_route_path(route: Route) -> String {
    let base = url::Url::parse(route.family().default_base()).unwrap();
    let mut path = base
        .path()
        .strip_prefix("/api")
        .unwrap_or(base.path())
        .to_string();
    for segment in route.segments() {
        path.push('/');
        match segment {
            Segment::Static(value) => path.push_str(value),
            Segment::Parameter => path.push_str("{}"),
        }
    }
    path
}

fn normalize_parameters(path: &str) -> String {
    let mut normalized = String::with_capacity(path.len());
    let mut in_parameter = false;
    for character in path.chars() {
        match character {
            '{' => {
                in_parameter = true;
                normalized.push_str("{}");
            },
            '}' => in_parameter = false,
            _ if !in_parameter => normalized.push(character),
            _ => {},
        }
    }
    normalized
}
