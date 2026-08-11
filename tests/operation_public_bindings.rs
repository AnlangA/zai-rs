//! External-crate compile contract for every frozen HTTP operation binding.
//!
//! Each row below declares its Rust request type, terminal method, response
//! type, and optional stream item exactly once. The macros use those same
//! tokens both to compile the real terminal and to derive the metadata checked
//! against `operations.json`.

#![allow(dead_code)]

use std::collections::{HashMap, HashSet};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct FrozenBinding {
    operation_id: String,
    response_mode: String,
    requires_done: bool,
    service_method: String,
    request_type: String,
    response_type: String,
    stream_item: Option<String>,
}

#[derive(Debug)]
struct PublicBinding {
    operation_id: &'static str,
    service_method: String,
    request_type: String,
    response_type: String,
    stream_item: Option<String>,
}

fn normalize_tokens(tokens: &str) -> String {
    let compact: String = tokens
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    compact.replace(',', ", ")
}

fn terminal_path(request: &str, method: &str) -> String {
    format!(
        "{}::{}",
        normalize_tokens(request),
        normalize_tokens(method)
    )
}

fn joined(left: String, right: String) -> String {
    if left == right {
        left
    } else {
        format!("{left} / {right}")
    }
}

fn single_binding(
    operation_id: &'static str,
    request: &str,
    method: &str,
    response: &str,
    stream_item: Option<&str>,
) -> PublicBinding {
    PublicBinding {
        operation_id,
        service_method: terminal_path(request, method),
        request_type: normalize_tokens(request),
        response_type: normalize_tokens(response),
        stream_item: stream_item.map(normalize_tokens),
    }
}

#[allow(clippy::too_many_arguments)]
fn dual_binding(
    operation_id: &'static str,
    first_request: &str,
    first_method: &str,
    first_response: &str,
    second_request: &str,
    second_method: &str,
    second_response: &str,
    stream_item: &str,
) -> PublicBinding {
    PublicBinding {
        operation_id,
        service_method: joined(
            terminal_path(first_request, first_method),
            terminal_path(second_request, second_method),
        ),
        request_type: joined(
            normalize_tokens(first_request),
            normalize_tokens(second_request),
        ),
        response_type: joined(
            normalize_tokens(first_response),
            normalize_tokens(second_response),
        ),
        stream_item: Some(normalize_tokens(stream_item)),
    }
}

macro_rules! assert_stream_item {
    ($response:ty, $item:ty) => {
        fn stream_item_is_public_and_exact() {
            fn assert_stream<S, T>()
            where
                S: futures_util::Stream<Item = zai_rs::ZaiResult<T>>,
            {
            }

            assert_stream::<$response, $item>();
        }
    };
}

macro_rules! binding_row {
    (single $module:ident {
        operation: $operation:literal;
        request: $request:ty;
        method: $method:ident;
        response: $response:ty;
    }) => {
        mod $module {
            pub(super) fn binding() -> super::PublicBinding {
                super::single_binding(
                    $operation,
                    stringify!($request),
                    stringify!($method),
                    stringify!($response),
                    None,
                )
            }

            async fn terminal(
                request: &$request,
                client: &zai_rs::ZaiClient,
            ) -> zai_rs::ZaiResult<$response> {
                request.$method(client).await
            }
        }
    };

    (single_with_args $module:ident {
        operation: $operation:literal;
        request: $request:ty;
        method: $method:ident;
        args: [$($argument:ident: $argument_type:ty),+ $(,)?];
        response: $response:ty;
    }) => {
        mod $module {
            pub(super) fn binding() -> super::PublicBinding {
                super::single_binding(
                    $operation,
                    stringify!($request),
                    stringify!($method),
                    stringify!($response),
                    None,
                )
            }

            async fn terminal(
                request: &$request,
                client: &zai_rs::ZaiClient,
                $($argument: $argument_type),+
            ) -> zai_rs::ZaiResult<$response> {
                request.$method(client, $($argument),+).await
            }
        }
    };

    (stream $module:ident {
        operation: $operation:literal;
        request: $request:ty;
        method: $method:ident;
        response: $response:ty;
        item: $item:ty;
    }) => {
        mod $module {
            pub(super) fn binding() -> super::PublicBinding {
                super::single_binding(
                    $operation,
                    stringify!($request),
                    stringify!($method),
                    stringify!($response),
                    Some(stringify!($item)),
                )
            }

            async fn terminal(
                request: &$request,
                client: &zai_rs::ZaiClient,
            ) -> zai_rs::ZaiResult<$response> {
                request.$method(client).await
            }

            assert_stream_item!($response, $item);
        }
    };

    (dual $module:ident {
        operation: $operation:literal;
        first: [$first_request:ty, $first_method:ident, $first_response:ty];
        second: [$second_request:ty, $second_method:ident, $second_response:ty];
        item: $item:ty;
    }) => {
        mod $module {
            pub(super) fn binding() -> super::PublicBinding {
                super::dual_binding(
                    $operation,
                    stringify!($first_request),
                    stringify!($first_method),
                    stringify!($first_response),
                    stringify!($second_request),
                    stringify!($second_method),
                    stringify!($second_response),
                    stringify!($item),
                )
            }

            async fn first_terminal(
                request: &$first_request,
                client: &zai_rs::ZaiClient,
            ) -> zai_rs::ZaiResult<$first_response> {
                request.$first_method(client).await
            }

            async fn second_terminal(
                request: &$second_request,
                client: &zai_rs::ZaiClient,
            ) -> zai_rs::ZaiResult<$second_response> {
                request.$second_method(client).await
            }

            assert_stream_item!($second_response, $item);
        }
    };

    (generic $module:ident {
        operation: $operation:literal;
        generics: [$($generic:ident),+ $(,)?];
        where: [$($bounds:tt)+];
        request: $request:ty;
        method: $method:ident;
        response: $response:ty;
    }) => {
        mod $module {
            pub(super) fn binding() -> super::PublicBinding {
                super::single_binding(
                    $operation,
                    stringify!($request),
                    stringify!($method),
                    stringify!($response),
                    None,
                )
            }

            async fn terminal<$($generic),+>(
                request: &$request,
                client: &zai_rs::ZaiClient,
            ) -> zai_rs::ZaiResult<$response>
            where
                $($bounds)+
            {
                request.$method(client).await
            }
        }
    };

    (generic_dual $module:ident {
        operation: $operation:literal;
        generics: [$($generic:ident),+ $(,)?];
        where: [$($bounds:tt)+];
        first: [$first_request:ty, $first_method:ident, $first_response:ty];
        second: [$second_request:ty, $second_method:ident, $second_response:ty];
        item: $item:ty;
    }) => {
        mod $module {
            pub(super) fn binding() -> super::PublicBinding {
                super::dual_binding(
                    $operation,
                    stringify!($first_request),
                    stringify!($first_method),
                    stringify!($first_response),
                    stringify!($second_request),
                    stringify!($second_method),
                    stringify!($second_response),
                    stringify!($item),
                )
            }

            async fn first_terminal<$($generic),+>(
                request: &$first_request,
                client: &zai_rs::ZaiClient,
            ) -> zai_rs::ZaiResult<$first_response>
            where
                $($bounds)+
            {
                request.$first_method(client).await
            }

            async fn second_terminal<$($generic),+>(
                request: &$second_request,
                client: &zai_rs::ZaiClient,
            ) -> zai_rs::ZaiResult<$second_response>
            where
                $($bounds)+
            {
                request.$second_method(client).await
            }

            assert_stream_item!($second_response, $item);
        }
    };
}

macro_rules! declare_bindings {
    ($( $kind:ident $module:ident { $($binding:tt)* })+) => {
        $(binding_row!($kind $module { $($binding)* });)+

        fn public_bindings() -> Vec<PublicBinding> {
            vec![$($module::binding()),+]
        }
    };
}

declare_bindings! {
    single agents_invoke {
        operation: "agents.invoke";
        request: zai_rs::agent::AgentInvokeRequest<zai_rs::agent::NonStreaming>;
        method: send_via;
        response: zai_rs::agent::AgentInvokeResponse;
    }
    single agents_async_result {
        operation: "agents.async_result";
        request: zai_rs::agent::AgentAsyncResultRequest;
        method: send_via;
        response: zai_rs::agent::AgentAsyncResult;
    }
    single agents_conversation {
        operation: "agents.conversation";
        request: zai_rs::agent::AgentConversationRequest;
        method: send_via;
        response: zai_rs::agent::AgentConversationResponse;
    }
    single applications_history {
        operation: "applications.history";
        request: zai_rs::services::applications::ApplicationHistoryRequest;
        method: send_via;
        response: zai_rs::services::applications::ApplicationHistoryResponse;
    }
    single applications_file_stats {
        operation: "applications.file_stats";
        request: zai_rs::services::applications::ApplicationFileStatsRequest;
        method: send_via;
        response: zai_rs::services::applications::ApplicationFileStatsResponse;
    }
    single applications_upload_file {
        operation: "applications.upload_file";
        request: zai_rs::services::applications::ApplicationFileUploadRequest;
        method: send_via;
        response: zai_rs::services::applications::ApplicationFileUploadResponse;
    }
    single applications_slice_info {
        operation: "applications.slice_info";
        request: zai_rs::services::applications::ApplicationSliceInfoRequest;
        method: send_via;
        response: zai_rs::services::applications::ApplicationSliceInfoResponse;
    }
    single applications_create_conversation {
        operation: "applications.create_conversation";
        request: zai_rs::services::applications::ApplicationConversationCreateRequest;
        method: send_via;
        response: zai_rs::services::applications::ApplicationConversationCreateResponse;
    }
    single applications_variables {
        operation: "applications.variables";
        request: zai_rs::services::applications::ApplicationVariablesRequest;
        method: send_via;
        response: zai_rs::services::applications::ApplicationVariablesResponse;
    }
    single applications_invoke {
        operation: "applications.invoke";
        request: zai_rs::services::applications::ApplicationInvokeRequest;
        method: send_via;
        response: zai_rs::services::applications::ApplicationInvokeResponse;
    }
    single knowledge_list_documents {
        operation: "knowledge.list_documents";
        request: zai_rs::knowledge::DocumentListRequest;
        method: send_via;
        response: zai_rs::knowledge::DocumentListResponse;
    }
    single knowledge_reembed_document {
        operation: "knowledge.reembed_document";
        request: zai_rs::knowledge::DocumentReembedRequest;
        method: send_via;
        response: zai_rs::knowledge::DocumentReembedResponse;
    }
    single knowledge_list_document_images {
        operation: "knowledge.list_document_images";
        request: zai_rs::knowledge::DocumentImageListRequest;
        method: send_via;
        response: zai_rs::knowledge::DocumentImageListResponse;
    }
    single knowledge_upload_document {
        operation: "knowledge.upload_document";
        request: zai_rs::knowledge::DocumentUploadRequest;
        method: send_via;
        response: zai_rs::knowledge::DocumentUploadResponse;
    }
    single knowledge_upload_document_url {
        operation: "knowledge.upload_document_url";
        request: zai_rs::knowledge::DocumentUrlUploadRequest;
        method: send_via;
        response: zai_rs::knowledge::DocumentUrlUploadResponse;
    }
    single knowledge_delete_document {
        operation: "knowledge.delete_document";
        request: zai_rs::knowledge::DocumentDeleteRequest;
        method: send_via;
        response: zai_rs::knowledge::DocumentDeleteResponse;
    }
    single knowledge_get_document {
        operation: "knowledge.get_document";
        request: zai_rs::knowledge::DocumentGetRequest;
        method: send_via;
        response: zai_rs::knowledge::DocumentGetResponse;
    }
    single knowledge_list {
        operation: "knowledge.list";
        request: zai_rs::knowledge::KnowledgeListRequest;
        method: send_via;
        response: zai_rs::knowledge::KnowledgeListResponse;
    }
    single knowledge_create {
        operation: "knowledge.create";
        request: zai_rs::knowledge::KnowledgeCreateRequest;
        method: send_via;
        response: zai_rs::knowledge::KnowledgeCreateResponse;
    }
    single knowledge_capacity {
        operation: "knowledge.capacity";
        request: zai_rs::knowledge::KnowledgeCapacityRequest;
        method: send_via;
        response: zai_rs::knowledge::KnowledgeCapacityResponse;
    }
    single knowledge_retrieve {
        operation: "knowledge.retrieve";
        request: zai_rs::knowledge::KnowledgeSearchRequest;
        method: send_via;
        response: zai_rs::knowledge::KnowledgeSearchResponse;
    }
    single knowledge_delete {
        operation: "knowledge.delete";
        request: zai_rs::knowledge::KnowledgeDeleteRequest;
        method: send_via;
        response: zai_rs::knowledge::KnowledgeDeleteResponse;
    }
    single knowledge_get {
        operation: "knowledge.get";
        request: zai_rs::knowledge::KnowledgeGetRequest;
        method: send_via;
        response: zai_rs::knowledge::KnowledgeGetResponse;
    }
    single knowledge_update {
        operation: "knowledge.update";
        request: zai_rs::knowledge::KnowledgeUpdateRequest;
        method: send_via;
        response: zai_rs::knowledge::KnowledgeUpdateResponse;
    }
    single assistants_invoke {
        operation: "assistants.invoke";
        request: zai_rs::services::assistants::AssistantInvokeRequest;
        method: send_via;
        response: zai_rs::services::assistants::AssistantInvokeResponse;
    }
    single assistants_conversations {
        operation: "assistants.conversations";
        request: zai_rs::services::assistants::AssistantConversationListRequest;
        method: send_via;
        response: zai_rs::services::assistants::AssistantConversationListResponse;
    }
    single assistants_list {
        operation: "assistants.list";
        request: zai_rs::services::assistants::AssistantListRequest;
        method: send_via;
        response: zai_rs::services::assistants::AssistantListResponse;
    }
    single tasks_get {
        operation: "tasks.get";
        request: zai_rs::model::async_chat_get::AsyncTaskGetRequest;
        method: send_via;
        response: zai_rs::model::async_chat_get::AsyncTaskResult;
    }
    generic chat_complete_async {
        operation: "chat.complete_async";
        generics: [N, M];
        where: [
            N: zai_rs::model::traits::ChatRequestModel
                + zai_rs::model::traits::AsyncChat
                + serde::Serialize,
            M: serde::Serialize,
            (N, M): zai_rs::model::traits::Bounded,
            zai_rs::model::chat_base_request::ChatBody<N, M>: serde::Serialize,
        ];
        request: zai_rs::model::async_chat::AsyncChatCompletion<N, M>;
        method: send_via;
        response: zai_rs::model::async_chat_get::AsyncResponse;
    }
    single images_generate_async {
        operation: "images.generate_async";
        request: zai_rs::services::images::AsyncImageGenerationRequest;
        method: send_via;
        response: zai_rs::model::async_chat_get::AsyncResponse;
    }
    generic_dual audio_synthesize {
        operation: "audio.synthesize";
        generics: [N];
        where: [N: zai_rs::model::traits::TextToAudio,];
        first: [
            zai_rs::model::text_to_audio::TextToAudioRequest<
                N,
                zai_rs::model::traits::StreamOff
            >,
            send_via,
            bytes::Bytes
        ];
        second: [
            zai_rs::model::text_to_audio::TextToAudioRequest<
                N,
                zai_rs::model::traits::StreamOn
            >,
            stream_via,
            zai_rs::model::text_to_audio::TextToAudioStream
        ];
        item: bytes::Bytes;
    }
    generic_dual audio_transcribe {
        operation: "audio.transcribe";
        generics: [N];
        where: [N: zai_rs::model::traits::AudioToText,];
        first: [
            zai_rs::model::audio_to_text::AudioToTextRequest<
                N,
                zai_rs::model::traits::StreamOff
            >,
            send_via,
            zai_rs::model::audio_to_text::AudioToTextResponse
        ];
        second: [
            zai_rs::model::audio_to_text::AudioToTextRequest<
                N,
                zai_rs::model::traits::StreamOn
            >,
            stream_via,
            zai_rs::model::audio_to_text::SpeechToTextStream
        ];
        item: zai_rs::model::audio_to_text::SpeechToTextEvent;
    }
    single batches_list {
        operation: "batches.list";
        request: zai_rs::batches::BatchListRequest;
        method: send_via;
        response: zai_rs::batches::BatchListResponse;
    }
    single batches_create {
        operation: "batches.create";
        request: zai_rs::batches::BatchCreateRequest;
        method: send_via;
        response: zai_rs::batches::BatchCreateResponse;
    }
    single batches_get {
        operation: "batches.get";
        request: zai_rs::batches::BatchGetRequest;
        method: send_via;
        response: zai_rs::batches::BatchGetResponse;
    }
    single batches_cancel {
        operation: "batches.cancel";
        request: zai_rs::batches::BatchCancelRequest;
        method: send_via;
        response: zai_rs::batches::BatchCancelResponse;
    }
    generic_dual chat_complete {
        operation: "chat.complete";
        generics: [N, M];
        where: [
            N: zai_rs::model::traits::ChatRequestModel
                + zai_rs::model::traits::Chat
                + serde::Serialize,
            M: serde::Serialize,
            (N, M): zai_rs::model::traits::Bounded,
            zai_rs::model::chat_base_request::ChatBody<N, M>: serde::Serialize,
        ];
        first: [
            zai_rs::model::chat::ChatCompletion<
                N,
                M,
                zai_rs::model::traits::StreamOff
            >,
            send_via,
            zai_rs::model::chat_base_response::ChatCompletionResponse
        ];
        second: [
            zai_rs::model::chat::ChatCompletion<
                N,
                M,
                zai_rs::model::traits::StreamOn
            >,
            stream_via,
            zai_rs::model::chat::ChatStream
        ];
        item: zai_rs::model::chat_stream_response::ChatStreamResponse;
    }
    single embeddings_create {
        operation: "embeddings.create";
        request: zai_rs::model::text_embedded::EmbeddingRequest;
        method: send_via;
        response: zai_rs::model::text_embedded::EmbeddingResponse;
    }
    single files_list {
        operation: "files.list";
        request: zai_rs::file::FileListRequest;
        method: send_via;
        response: zai_rs::file::FileListResponse;
    }
    single files_upload {
        operation: "files.upload";
        request: zai_rs::file::FileUploadRequest;
        method: send_via;
        response: zai_rs::file::FileUploadResponse;
    }
    single files_ocr {
        operation: "files.ocr";
        request: zai_rs::model::ocr::OcrRequest;
        method: send_via;
        response: zai_rs::model::ocr::OcrResponse;
    }
    single files_parse {
        operation: "files.parse";
        request: zai_rs::tool::FileParseRequest;
        method: send_via;
        response: zai_rs::tool::FileParserCreateResponse;
    }
    single_with_args files_parse_result {
        operation: "files.parse_result";
        request: zai_rs::tool::FileParseResultRequest;
        method: get_result_via;
        args: [format: zai_rs::tool::FormatType];
        response: zai_rs::tool::FileParseResultResponse;
    }
    single files_parse_sync {
        operation: "files.parse_sync";
        request: zai_rs::file::FileParseSyncRequest;
        method: send_via;
        response: zai_rs::file::FileResponse;
    }
    single files_delete {
        operation: "files.delete";
        request: zai_rs::file::FileDeleteRequest;
        method: send_via;
        response: zai_rs::file::FileDeleteResponse;
    }
    dual files_content {
        operation: "files.content";
        first: [
            zai_rs::file::FileContentRequest,
            send_via,
            zai_rs::file::ByteStream
        ];
        second: [
            zai_rs::file::FileContentRequest,
            stream_via,
            zai_rs::file::FileContentStream
        ];
        item: bytes::Bytes;
    }
    generic images_generate {
        operation: "images.generate";
        generics: [N];
        where: [N: zai_rs::model::traits::ImageGen,];
        request: zai_rs::model::gen_image::ImageGenRequest<N>;
        method: send_via;
        response: zai_rs::model::gen_image::ImageResponse;
    }
    single tools_parse_layout {
        operation: "tools.parse_layout";
        request: zai_rs::services::tools::LayoutParsingRequest;
        method: send_via;
        response: zai_rs::services::tools::LayoutParsingResponse;
    }
    single moderation_check {
        operation: "moderation.check";
        request: zai_rs::model::moderation::Moderation;
        method: send_via;
        response: zai_rs::model::moderation::ModerationResponse;
    }
    single tools_read_document {
        operation: "tools.read_document";
        request: zai_rs::services::tools::ReaderRequest;
        method: send_via;
        response: zai_rs::services::tools::ReaderResponse;
    }
    single rerank_create {
        operation: "rerank.create";
        request: zai_rs::model::text_rerank::RerankRequest;
        method: send_via;
        response: zai_rs::model::text_rerank::RerankResponse;
    }
    single tokenizer_count {
        operation: "tokenizer.count";
        request: zai_rs::model::text_tokenizer::TokenizerRequest;
        method: send_via;
        response: zai_rs::model::text_tokenizer::TokenizerResponse;
    }
    generic videos_generate {
        operation: "videos.generate";
        generics: [N];
        where: [N: zai_rs::model::traits::VideoGen,];
        request: zai_rs::model::gen_video_async::VideoGenRequest<N>;
        method: send_via;
        response: zai_rs::model::async_chat_get::AsyncResponse;
    }
    generic audio_clone_voice {
        operation: "audio.clone_voice";
        generics: [N];
        where: [N: zai_rs::model::traits::VoiceClone,];
        request: zai_rs::model::voice_clone::VoiceCloneRequest<N>;
        method: send_via;
        response: zai_rs::model::voice_clone::VoiceCloneResponse;
    }
    single audio_delete_voice {
        operation: "audio.delete_voice";
        request: zai_rs::model::voice_delete::VoiceDeleteRequest;
        method: send_via;
        response: zai_rs::model::voice_delete::VoiceDeleteResponse;
    }
    single audio_list_voices {
        operation: "audio.list_voices";
        request: zai_rs::model::voice_list::VoiceListRequest;
        method: send_via;
        response: zai_rs::model::voice_list::VoiceListResponse;
    }
    single tools_web_search {
        operation: "tools.web_search";
        request: zai_rs::tool::WebSearchRequest;
        method: send_via;
        response: zai_rs::tool::WebSearchResponse;
    }
    stream zrag_chat {
        operation: "zrag.chat";
        request: zai_rs::zrag::ZragChatRequest;
        method: stream_via;
        response: zai_rs::zrag::ZragEventStream;
        item: zai_rs::zrag::AgentStreamEvent;
    }
    single zrag_retrieve {
        operation: "zrag.retrieve";
        request: zai_rs::zrag::ZragRetrieveRequest;
        method: send_via;
        response: zai_rs::zrag::ZragRetrieveResponse;
    }
}

fn frozen_bindings() -> Vec<FrozenBinding> {
    serde_json::from_str(include_str!("../spec/contracts/operations.json")).unwrap()
}

#[test]
fn all_frozen_operations_have_exactly_one_compiled_public_binding() {
    let frozen = frozen_bindings();
    let public = public_bindings();
    assert_eq!(frozen.len(), 59, "unexpected frozen operation count");
    assert_eq!(public.len(), 59, "unexpected public binding count");

    let mut by_operation: HashMap<_, _> = frozen
        .iter()
        .map(|binding| (binding.operation_id.as_str(), binding))
        .collect();
    assert_eq!(
        by_operation.len(),
        frozen.len(),
        "duplicate frozen operation id"
    );

    let unique_bindings: HashSet<_> = public.iter().map(|binding| binding.operation_id).collect();
    assert_eq!(
        unique_bindings.len(),
        public.len(),
        "duplicate public binding"
    );

    const REMOVED_IDENTIFIERS: &[&str] = &[
        "AgentEvent",
        "AgentEventStream",
        "AsyncChatRequest",
        "AudioByteStream",
        "ChatEvent",
        "ChatEventStream",
        "ChatRequest",
        "ChatResponse",
        "ImageGenerationRequest",
        "ImageGenerationResponse",
        "SpeechToTextRequest",
        "SpeechToTextResponse",
        "TextToSpeechRequest",
        "VideoGenerationRequest",
    ];

    for expected in &public {
        let actual = by_operation
            .remove(expected.operation_id)
            .unwrap_or_else(|| panic!("missing frozen binding for {}", expected.operation_id));
        assert_eq!(
            actual.service_method, expected.service_method,
            "{} service_method",
            actual.operation_id
        );
        assert_eq!(
            actual.request_type, expected.request_type,
            "{} request_type",
            actual.operation_id
        );
        assert_eq!(
            actual.response_type, expected.response_type,
            "{} response_type",
            actual.operation_id
        );
        assert_eq!(
            actual.stream_item.as_deref(),
            expected.stream_item.as_deref(),
            "{} stream_item",
            actual.operation_id
        );

        for terminal in expected.service_method.split(" / ") {
            assert!(
                terminal.ends_with("::send_via")
                    || terminal.ends_with("::stream_via")
                    || terminal.ends_with("::get_result_via"),
                "{} does not name a public request terminal: {terminal}",
                actual.operation_id
            );
        }

        for value in [
            actual.service_method.as_str(),
            actual.request_type.as_str(),
            actual.response_type.as_str(),
        ]
        .into_iter()
        .chain(actual.stream_item.as_deref())
        {
            for path in value.split(" / ") {
                assert!(
                    path.starts_with("zai_rs::") || path.starts_with("bytes::"),
                    "{} binding is not an external-crate-qualified path: {path}",
                    actual.operation_id
                );
            }
            assert!(
                !value.contains("client."),
                "{} still names a removed client facade: {value}",
                actual.operation_id
            );
            for identifier in value
                .split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
            {
                assert!(
                    !REMOVED_IDENTIFIERS.contains(&identifier),
                    "{} still names removed public identifier {identifier}",
                    actual.operation_id
                );
            }
        }
    }

    assert!(
        by_operation.is_empty(),
        "frozen operations without a compiled binding: {by_operation:?}"
    );
    assert_eq!(
        frozen
            .iter()
            .filter(|binding| binding.stream_item.is_some())
            .count(),
        5,
        "only the five real public stream bindings may name stream items"
    );
}

#[test]
fn special_terminal_metadata_cannot_regress_to_invented_apis() {
    let public = public_bindings();
    let public_by_id: HashMap<_, _> = public
        .iter()
        .map(|binding| (binding.operation_id, binding))
        .collect();

    let agent = public_by_id["agents.invoke"];
    assert_eq!(agent.stream_item, None);
    assert!(!agent.service_method.contains("stream"));
    assert!(!agent.response_type.contains("AgentEvent"));

    let parser = public_by_id["files.parse_result"];
    assert!(parser.service_method.ends_with("::get_result_via"));

    let frozen = frozen_bindings();
    let frozen_by_id: HashMap<_, _> = frozen
        .iter()
        .map(|binding| (binding.operation_id.as_str(), binding))
        .collect();
    let tts = frozen_by_id["audio.synthesize"];
    assert_eq!(tts.response_mode, "binary_or_stream");
    assert!(
        tts.requires_done,
        "the TTS streaming branch requires a terminal [DONE] marker"
    );
}
