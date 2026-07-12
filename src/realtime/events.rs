//! Typed events from the official GLM-Realtime protocol.
//!
//! The core wire shapes are pinned to
//! `spec/upstream/asyncapi-2026-07-11.json`; the guide-only conversation,
//! output-item, content-part, and rate-limit events are modeled as well.

use serde::{Deserialize, Serialize};

use super::protocol::{ItemContent, RealtimeConversationItem, RealtimeResponse, SessionConfig};

/// Conversation metadata carried by `conversation.created`.
#[derive(Debug, Clone, Deserialize)]
pub struct RealtimeConversation {
    /// Conversation id.
    pub id: String,
    /// Protocol object discriminator (`"realtime.conversation"`).
    pub object: String,
}

/// Body of a server `error` event.
#[derive(Debug, Clone, Deserialize)]
pub struct ServerErrorBody {
    /// Error type, e.g. `"invalid_request_error"`, `"server_error"`.
    #[serde(rename = "type")]
    pub type_: String,
    /// Machine-readable error code (string per the GLM protocol).
    #[serde(default)]
    pub code: Option<String>,
    /// Human-readable message.
    pub message: String,
}

/// Open session metadata carried by `session.created`.
///
/// The server-created shape contains identifiers and effective defaults that
/// are not valid fields in an outbound [`SessionConfig`]. Known fields are
/// exposed directly while future additions are preserved in [`Self::extra`].
#[derive(Debug, Clone, Deserialize)]
pub struct RealtimeSessionInfo {
    /// Server-assigned session id.
    #[serde(default)]
    pub id: Option<String>,
    /// Protocol object discriminator, normally `"realtime.session"`.
    #[serde(default)]
    pub object: Option<String>,
    /// Effective model id.
    #[serde(default)]
    pub model: Option<String>,
    /// Effective output modalities.
    #[serde(default)]
    pub modalities: Vec<String>,
    /// Effective voice id. Kept open because older servers may report
    /// `"default"` rather than a configurable voice id.
    #[serde(default)]
    pub voice: Option<String>,
    /// Effective input audio format.
    #[serde(default)]
    pub input_audio_format: Option<String>,
    /// Effective output audio format.
    #[serde(default)]
    pub output_audio_format: Option<String>,
    /// Effective sampling temperature.
    #[serde(default)]
    pub temperature: Option<f64>,
    /// Additional protocol fields added by the server.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// One quota entry carried by `rate_limits.updated`.
#[derive(Debug, Clone, Deserialize)]
pub struct RealtimeRateLimit {
    /// Quota name, such as `"requests"`.
    pub name: String,
    /// Maximum allowance in the current window.
    pub limit: u64,
    /// Remaining allowance in the current window.
    pub remaining: u64,
    /// Seconds until the quota window resets.
    pub reset_seconds: f64,
}

// ---------------------------------------------------------------------------
// Client events (sent client → server). Serialize-only.
// ---------------------------------------------------------------------------

/// A client event, tagged by `type` to match the official event names.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ClientEvent {
    /// `session.update` — set the session defaults (formats, VAD, tools, …).
    #[serde(rename = "session.update")]
    SessionUpdate {
        /// Optional client-side event id for correlation.
        #[serde(skip_serializing_if = "Option::is_none")]
        event_id: Option<String>,
        /// New session configuration to apply.
        session: SessionConfig,
    },

    /// `input_audio_buffer.append` — upload base64 WAV audio.
    #[serde(rename = "input_audio_buffer.append")]
    InputAudioBufferAppend {
        /// Base64-encoded audio payload.
        audio: String,
        /// Optional client-side timestamp (ms).
        #[serde(skip_serializing_if = "Option::is_none")]
        client_timestamp: Option<i64>,
    },

    /// `input_audio_buffer.append_video_frame` — upload a base64 JPEG frame.
    #[serde(rename = "input_audio_buffer.append_video_frame")]
    InputAudioBufferAppendVideoFrame {
        /// Base64-encoded JPEG video frame.
        video_frame: String,
        /// Optional client-side timestamp (ms).
        #[serde(skip_serializing_if = "Option::is_none")]
        client_timestamp: Option<i64>,
    },

    /// `input_audio_buffer.commit` — commit buffered audio for inference.
    #[serde(rename = "input_audio_buffer.commit")]
    InputAudioBufferCommit {
        /// Optional client-side timestamp (ms).
        #[serde(skip_serializing_if = "Option::is_none")]
        client_timestamp: Option<i64>,
    },

    /// `input_audio_buffer.clear` — clear the buffer.
    #[serde(rename = "input_audio_buffer.clear")]
    InputAudioBufferClear,

    /// `conversation.item.create` — inject a text message or function-call
    /// output into the conversation history.
    #[serde(rename = "conversation.item.create")]
    ConversationItemCreate {
        /// Optional client-side event id for correlation.
        #[serde(skip_serializing_if = "Option::is_none")]
        event_id: Option<String>,
        /// The conversation item to insert.
        item: RealtimeConversationItem,
    },

    /// `conversation.item.delete` — remove an item from conversation history.
    #[serde(rename = "conversation.item.delete")]
    ConversationItemDelete {
        /// Optional client-side event id for correlation.
        #[serde(skip_serializing_if = "Option::is_none")]
        event_id: Option<String>,
        /// Optional client-side timestamp (ms).
        #[serde(skip_serializing_if = "Option::is_none")]
        client_timestamp: Option<i64>,
        /// Id of the item to remove.
        item_id: String,
    },

    /// `conversation.item.retrieve` — request one conversation-history item.
    #[serde(rename = "conversation.item.retrieve")]
    ConversationItemRetrieve {
        /// Optional client-side event id for correlation.
        #[serde(skip_serializing_if = "Option::is_none")]
        event_id: Option<String>,
        /// Optional client-side timestamp (ms).
        #[serde(skip_serializing_if = "Option::is_none")]
        client_timestamp: Option<i64>,
        /// Id of the item to retrieve.
        item_id: String,
    },

    /// `response.create` — trigger model inference.
    #[serde(rename = "response.create")]
    ResponseCreate {
        /// Optional client-side timestamp (ms).
        #[serde(skip_serializing_if = "Option::is_none")]
        client_timestamp: Option<i64>,
    },

    /// `response.cancel` — cancel the in-flight response (interruption).
    #[serde(rename = "response.cancel")]
    ResponseCancel {
        /// Optional client-side timestamp (ms).
        #[serde(skip_serializing_if = "Option::is_none")]
        client_timestamp: Option<i64>,
    },
}

// ---------------------------------------------------------------------------
// Server events (received server → client). Deserialize-only.
// ---------------------------------------------------------------------------

/// A server event, tagged by `type`.
///
/// Extra fields on a known event are ignored. Unknown event types deserialize
/// as [`ServerEvent::Unknown`] so a newer server cannot tear down an otherwise
/// healthy session. The enum is non-exhaustive because the server protocol can
/// add event types independently of this crate.
#[non_exhaustive]
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ServerEvent {
    /// Server-side error (most are recoverable; the session stays open).
    #[serde(rename = "error")]
    Error {
        /// Error detail body.
        error: ServerErrorBody,
    },

    /// `session.created` — session established with effective server defaults.
    #[serde(rename = "session.created")]
    SessionCreated {
        /// Server-created session metadata.
        session: RealtimeSessionInfo,
    },

    /// `session.updated` — confirms a `session.update`.
    #[serde(rename = "session.updated")]
    SessionUpdated {
        /// Effective server-side session configuration.
        session: SessionConfig,
    },

    /// `conversation.created` — one per session.
    #[serde(rename = "conversation.created")]
    ConversationCreated {
        /// Newly created conversation metadata.
        conversation: RealtimeConversation,
    },

    /// `conversation.item.created`.
    #[serde(rename = "conversation.item.created")]
    ConversationItemCreated {
        /// The conversation item that was created.
        item: RealtimeConversationItem,
    },

    /// `conversation.item.deleted`.
    #[serde(rename = "conversation.item.deleted")]
    ConversationItemDeleted {
        /// Id of the deleted conversation item.
        item_id: String,
    },

    /// `conversation.item.retrieved`.
    #[serde(rename = "conversation.item.retrieved")]
    ConversationItemRetrieved {
        /// Retrieved conversation item.
        item: RealtimeConversationItem,
    },

    /// `conversation.item.input_audio_transcription.completed`.
    #[serde(rename = "conversation.item.input_audio_transcription.completed")]
    InputAudioTranscriptionCompleted {
        /// Id of the transcribed audio item.
        item_id: String,
        /// Index of the audio content part within the item.
        #[serde(default)]
        content_index: Option<u64>,
        /// Transcribed text.
        transcript: String,
    },

    /// `conversation.item.input_audio_transcription.failed`.
    #[serde(rename = "conversation.item.input_audio_transcription.failed")]
    InputAudioTranscriptionFailed {
        /// Id of the audio item whose transcription failed.
        item_id: String,
        /// Index of the audio content part within the item.
        #[serde(default)]
        content_index: Option<u64>,
        /// Error detail body.
        error: ServerErrorBody,
    },

    /// `input_audio_buffer.committed`.
    #[serde(rename = "input_audio_buffer.committed")]
    InputAudioBufferCommitted {
        /// Id of the committed audio item.
        item_id: String,
    },

    /// `input_audio_buffer.cleared`.
    #[serde(rename = "input_audio_buffer.cleared")]
    InputAudioBufferCleared,

    /// `input_audio_buffer.speech_started` (server-VAD only).
    #[serde(rename = "input_audio_buffer.speech_started")]
    InputAudioBufferSpeechStarted {
        /// Millisecond offset at which speech started, when supplied.
        #[serde(default)]
        audio_start_ms: Option<u64>,
        /// Id of the user item created for this speech turn, when supplied.
        #[serde(default)]
        item_id: Option<String>,
    },

    /// `input_audio_buffer.speech_stopped` (server-VAD only).
    #[serde(rename = "input_audio_buffer.speech_stopped")]
    InputAudioBufferSpeechStopped {
        /// Millisecond offset at which speech stopped, when supplied.
        #[serde(default)]
        audio_end_ms: Option<u64>,
        /// Id of the user item created for this speech turn, when supplied.
        #[serde(default)]
        item_id: Option<String>,
    },

    /// `response.output_item.added` — a response output item began streaming.
    #[serde(rename = "response.output_item.added")]
    ResponseOutputItemAdded {
        /// Id of the response that owns the item.
        response_id: String,
        /// Index of the item within the response output.
        output_index: u64,
        /// Newly added output item.
        item: RealtimeConversationItem,
    },

    /// `response.output_item.done` — a response output item finished.
    #[serde(rename = "response.output_item.done")]
    ResponseOutputItemDone {
        /// Id of the response that owns the item.
        response_id: String,
        /// Index of the item within the response output.
        output_index: u64,
        /// Final output item.
        item: RealtimeConversationItem,
    },

    /// `response.content_part.added` — a content part began streaming.
    #[serde(rename = "response.content_part.added")]
    ResponseContentPartAdded {
        /// Id of the response that owns the content part.
        response_id: String,
        /// Id of the output item that owns the content part.
        item_id: String,
        /// Index of the output item within the response.
        output_index: u64,
        /// Index of the content part within the output item.
        content_index: u64,
        /// Newly added content part.
        part: ItemContent,
    },

    /// `response.content_part.done` — a content part finished streaming.
    #[serde(rename = "response.content_part.done")]
    ResponseContentPartDone {
        /// Id of the response that owns the content part.
        response_id: String,
        /// Id of the output item that owns the content part.
        item_id: String,
        /// Index of the output item within the response.
        output_index: u64,
        /// Index of the content part within the output item.
        content_index: u64,
        /// Final content part.
        part: ItemContent,
    },

    /// `response.created`.
    #[serde(rename = "response.created")]
    ResponseCreated {
        /// The response object.
        response: RealtimeResponse,
    },

    /// `response.done` — final state + usage. Always emitted.
    #[serde(rename = "response.done")]
    ResponseDone {
        /// The final response object.
        response: RealtimeResponse,
    },

    /// `response.cancelled` — confirms cancellation and carries final state.
    #[serde(rename = "response.cancelled")]
    ResponseCancelled {
        /// The cancelled response object.
        response: RealtimeResponse,
    },

    /// `response.text.delta` — incremental model text.
    #[serde(rename = "response.text.delta")]
    ResponseTextDelta {
        /// Id of the response this text belongs to.
        response_id: String,
        /// Id of the output item.
        item_id: String,
        /// Index of the output item within the response, when supplied.
        #[serde(default)]
        output_index: Option<u64>,
        /// Index of the content part within the item, when supplied.
        #[serde(default)]
        content_index: Option<u64>,
        /// Incremental text content.
        delta: String,
    },

    /// `response.text.done` — final text for one content part.
    #[serde(rename = "response.text.done")]
    ResponseTextDone {
        /// Id of the response this text belongs to.
        response_id: String,
        /// Id of the output item.
        item_id: String,
        /// Index of the output item within the response, when supplied.
        #[serde(default)]
        output_index: Option<u64>,
        /// Index of the content part within the item, when supplied.
        #[serde(default)]
        content_index: Option<u64>,
        /// Complete text, when supplied by the server.
        #[serde(default)]
        text: Option<String>,
    },

    /// `response.audio.delta` — base64 24 kHz, mono PCM chunk.
    #[serde(rename = "response.audio.delta")]
    ResponseAudioDelta {
        /// Id of the response this chunk belongs to.
        response_id: String,
        /// Id of the output item.
        item_id: String,
        /// Index of the output item within the response, when supplied.
        #[serde(default)]
        output_index: Option<u64>,
        /// Index of the content part within the item, when supplied.
        #[serde(default)]
        content_index: Option<u64>,
        /// Base64-encoded audio delta.
        delta: String,
    },

    /// `response.audio.done`.
    #[serde(rename = "response.audio.done")]
    ResponseAudioDone {
        /// Id of the response that finished.
        response_id: String,
        /// Id of the output item.
        item_id: String,
        /// Index of the output item within the response, when supplied.
        #[serde(default)]
        output_index: Option<u64>,
        /// Index of the content part within the item, when supplied.
        #[serde(default)]
        content_index: Option<u64>,
    },

    /// `response.audio_transcript.delta` — incremental transcript text.
    #[serde(rename = "response.audio_transcript.delta")]
    ResponseAudioTranscriptDelta {
        /// Id of the response this delta belongs to.
        response_id: String,
        /// Id of the output item.
        item_id: String,
        /// Index of the output item within the response, when supplied.
        #[serde(default)]
        output_index: Option<u64>,
        /// Index of the content part within the item, when supplied.
        #[serde(default)]
        content_index: Option<u64>,
        /// Incremental transcript text.
        delta: String,
    },

    /// `response.audio_transcript.done` — final transcript.
    #[serde(rename = "response.audio_transcript.done")]
    ResponseAudioTranscriptDone {
        /// Id of the response whose transcript completed.
        response_id: String,
        /// Id of the output item.
        item_id: String,
        /// Index of the output item within the response, when supplied.
        #[serde(default)]
        output_index: Option<u64>,
        /// Index of the content part within the item, when supplied.
        #[serde(default)]
        content_index: Option<u64>,
        /// Final transcript text.
        transcript: String,
    },

    /// `response.function_call_arguments.done` — completed tool call.
    #[serde(rename = "response.function_call_arguments.done")]
    ResponseFunctionCallArgumentsDone {
        /// Id of the response that produced the call.
        response_id: String,
        /// Index of the output item within the response, when supplied.
        #[serde(default)]
        output_index: Option<u64>,
        /// Name of the function/tool to invoke.
        name: String,
        /// JSON-encoded arguments for the call.
        arguments: String,
    },

    /// `response.function_call.simple_browser` — video link triggered search.
    #[serde(rename = "response.function_call.simple_browser")]
    ResponseFunctionCallSimpleBrowser {
        /// Built-in function name (currently `"simple_browser"`).
        name: String,
        /// Optional server search metadata. This beta payload is open-ended in
        /// the upstream schema, so preserve it without inventing a closed type.
        #[serde(default)]
        session: Option<serde_json::Value>,
    },

    /// `rate_limits.updated` — current request quota information.
    #[serde(rename = "rate_limits.updated")]
    RateLimitsUpdated {
        /// Current limits reported by the server.
        rate_limits: Vec<RealtimeRateLimit>,
    },

    /// `heartbeat` — keepalive (every ~30s).
    #[serde(rename = "heartbeat")]
    Heartbeat,

    /// A valid event type introduced by a newer server.
    ///
    /// The event payload is intentionally not retained because it has no
    /// stable schema yet. Applications should use a wildcard match arm and
    /// upgrade the crate when they need the new event.
    #[serde(other)]
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::realtime::{InputAudioNoiseReduction, NoiseReductionType, RealtimeVoice};

    #[test]
    fn session_update_serializes_to_official_shape() {
        let session = SessionConfig {
            model: Some("glm-realtime-flash".to_string()),
            voice: Some(RealtimeVoice::FemaleTianmei),
            temperature: Some(0.7),
            max_response_output_tokens: Some(512),
            input_audio_noise_reduction: Some(InputAudioNoiseReduction::new(
                NoiseReductionType::NearField,
            )),
            ..SessionConfig::default()
        };
        let ev = ClientEvent::SessionUpdate {
            event_id: Some("evt_1".into()),
            session,
        };
        let json = serde_json::to_value(&ev).unwrap();
        assert_eq!(json["type"], "session.update");
        assert_eq!(json["event_id"], "evt_1");
        assert_eq!(json["session"]["input_audio_format"], "wav");
        assert_eq!(json["session"]["output_audio_format"], "pcm");
        assert_eq!(json["session"]["turn_detection"]["type"], "client_vad");
        assert_eq!(json["session"]["model"], "glm-realtime-flash");
        assert_eq!(
            json["session"]["modalities"],
            serde_json::json!(["text", "audio"])
        );
        assert_eq!(json["session"]["temperature"], 0.7);
        assert_eq!(json["session"]["max_response_output_tokens"], "512");
        assert_eq!(json["session"]["voice"], "female-tianmei");
        assert_eq!(
            json["session"]["input_audio_noise_reduction"]["type"],
            "near_field"
        );
        assert_eq!(json["session"]["beta_fields"]["chat_mode"], "audio");
        assert_eq!(
            json["session"]["turn_detection"],
            serde_json::json!({ "type": "client_vad" })
        );
    }

    #[test]
    fn session_updated_accepts_server_infinite_token_default() {
        let event: ServerEvent = serde_json::from_str(
            r#"{"type":"session.updated","session":{"input_audio_format":"wav","output_audio_format":"pcm","turn_detection":{"type":"server_vad","create_response":true},"max_response_output_tokens":"inf","beta_fields":{"chat_mode":"audio"}}}"#,
        )
        .unwrap();
        assert!(matches!(
            event,
            ServerEvent::SessionUpdated { session }
                if session.max_response_output_tokens.is_none()
                    && session.turn_detection.create_response == Some(true)
        ));
    }

    #[test]
    fn audio_append_round_trips() {
        let ev = ClientEvent::InputAudioBufferAppend {
            audio: "UklGRiQ".into(),
            client_timestamp: Some(1731999464667),
        };
        let json = serde_json::to_string(&ev).unwrap();
        assert!(json.contains("\"type\":\"input_audio_buffer.append\""));
        assert!(json.contains("\"audio\":\"UklGRiQ\""));
        assert!(json.contains("\"client_timestamp\":1731999464667"));
    }

    #[test]
    fn conversation_history_commands_use_official_shapes() {
        let delete = ClientEvent::ConversationItemDelete {
            event_id: Some("evt_delete".into()),
            client_timestamp: Some(42),
            item_id: "item_1".into(),
        };
        let json = serde_json::to_value(delete).unwrap();
        assert_eq!(json["type"], "conversation.item.delete");
        assert_eq!(json["item_id"], "item_1");
        assert_eq!(json["client_timestamp"], 42);

        let retrieve = ClientEvent::ConversationItemRetrieve {
            event_id: None,
            client_timestamp: None,
            item_id: "item_2".into(),
        };
        let json = serde_json::to_value(retrieve).unwrap();
        assert_eq!(json["type"], "conversation.item.retrieve");
        assert_eq!(json["item_id"], "item_2");
        assert!(json.get("event_id").is_none());
    }

    #[test]
    fn server_audio_delta_parses_official_example() {
        // Trimmed shape of the official response.audio.delta example.
        let raw = r#"{"event_id":"event89","type":"response.audio.delta","client_timestamp":1737454096061,"response_id":"respbc50304acdea479b8bd55efd5346dbdf","item_id":"item_1","output_index":0,"content_index":0,"delta":"+w6hBu39"}"#;
        let ev: ServerEvent = serde_json::from_str(raw).unwrap();
        match ev {
            ServerEvent::ResponseAudioDelta {
                response_id, delta, ..
            } => {
                assert_eq!(response_id, "respbc50304acdea479b8bd55efd5346dbdf");
                assert_eq!(delta, "+w6hBu39");
            },
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn server_response_done_parses_official_example() {
        let raw = r#"{"event_id":"eventb94","type":"response.done","client_timestamp":1739001415611,"response":{"id":"respee64945eafb44facac88cea6f9de86f5","object":"realtime.response","status":"completed","usage":{"total_tokens":7,"input_tokens":4,"output_tokens":3,"input_token_details":{"text_tokens":4,"audio_tokens":0},"output_token_details":{"text_tokens":3,"audio_tokens":0}}}}"#;
        let ev: ServerEvent = serde_json::from_str(raw).unwrap();
        match ev {
            ServerEvent::ResponseDone { response } => {
                assert_eq!(response.status, "completed");
                let usage = response.usage.unwrap();
                assert_eq!(usage.total_tokens, 7);
                assert_eq!(usage.output_tokens, 3);
            },
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn server_text_events_preserve_consumable_text() {
        let delta: ServerEvent = serde_json::from_str(
            r#"{"type":"response.text.delta","response_id":"resp_1","item_id":"item_1","output_index":0,"content_index":1,"delta":"你好"}"#,
        )
        .unwrap();
        match delta {
            ServerEvent::ResponseTextDelta {
                response_id,
                item_id,
                output_index,
                content_index,
                delta,
            } => {
                assert_eq!(response_id, "resp_1");
                assert_eq!(item_id, "item_1");
                assert_eq!(output_index, Some(0));
                assert_eq!(content_index, Some(1));
                assert_eq!(delta, "你好");
            },
            _ => panic!("wrong variant"),
        }

        let done: ServerEvent = serde_json::from_str(
            r#"{"type":"response.text.done","response_id":"resp_1","item_id":"item_1","text":"你好！"}"#,
        )
        .unwrap();
        assert!(matches!(
            done,
            ServerEvent::ResponseTextDone {
                text: Some(text),
                ..
            } if text == "你好！"
        ));
    }

    #[test]
    fn server_cancelled_event_preserves_final_response() {
        let event: ServerEvent = serde_json::from_str(
            r#"{"type":"response.cancelled","response":{"id":"resp_1","object":"realtime.response","status":"cancelled"}}"#,
        )
        .unwrap();
        assert!(matches!(
            event,
            ServerEvent::ResponseCancelled { response }
                if response.id == "resp_1" && response.status == "cancelled"
        ));
    }

    #[test]
    fn server_events_preserve_required_correlation_fields() {
        let conversation: ServerEvent = serde_json::from_str(
            r#"{"type":"conversation.created","conversation":{"id":"conv_1","object":"realtime.conversation"}}"#,
        )
        .unwrap();
        assert!(matches!(
            conversation,
            ServerEvent::ConversationCreated { conversation }
                if conversation.id == "conv_1"
                    && conversation.object == "realtime.conversation"
        ));

        let transcript: ServerEvent = serde_json::from_str(
            r#"{"type":"response.audio_transcript.delta","response_id":"resp_1","item_id":"item_1","output_index":2,"content_index":3,"delta":"hello"}"#,
        )
        .unwrap();
        assert!(matches!(
            transcript,
            ServerEvent::ResponseAudioTranscriptDelta {
                response_id,
                item_id,
                output_index: Some(2),
                content_index: Some(3),
                delta,
            } if response_id == "resp_1" && item_id == "item_1" && delta == "hello"
        ));

        let function_call: ServerEvent = serde_json::from_str(
            r#"{"type":"response.function_call_arguments.done","response_id":"resp_1","output_index":1,"name":"weather","arguments":"{}"}"#,
        )
        .unwrap();
        assert!(matches!(
            function_call,
            ServerEvent::ResponseFunctionCallArgumentsDone {
                output_index: Some(1),
                name,
                ..
            } if name == "weather"
        ));

        let browser: ServerEvent = serde_json::from_str(
            r#"{"type":"response.function_call.simple_browser","name":"simple_browser","session":{"beta_fields":{"simple_browser":{"description":"searching"}}}}"#,
        )
        .unwrap();
        assert!(matches!(
            browser,
            ServerEvent::ResponseFunctionCallSimpleBrowser {
                name,
                session: Some(_),
            } if name == "simple_browser"
        ));

        assert!(
            serde_json::from_str::<ServerEvent>(
                r#"{"type":"response.audio.delta","response_id":"resp_1","delta":"AA=="}"#
            )
            .is_err()
        );
    }

    #[test]
    fn server_error_parses() {
        let raw = r#"{"event_id":"event_890","type":"error","error":{"type":"invalid_request_error","code":"invalid_event","message":"The 'type' field is missing."}}"#;
        let ev: ServerEvent = serde_json::from_str(raw).unwrap();
        match ev {
            ServerEvent::Error { error } => {
                assert_eq!(error.code.as_deref(), Some("invalid_event"));
            },
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn guide_only_conversation_and_lifecycle_events_parse() {
        let created: ServerEvent = serde_json::from_str(
            r#"{"type":"session.created","session":{"id":"sess_1","object":"realtime.session","voice":"default","future_field":true}}"#,
        )
        .unwrap();
        assert!(matches!(
            created,
            ServerEvent::SessionCreated { session }
                if session.id.as_deref() == Some("sess_1")
                    && session.extra.contains_key("future_field")
        ));

        let deleted: ServerEvent =
            serde_json::from_str(r#"{"type":"conversation.item.deleted","item_id":"item_1"}"#)
                .unwrap();
        assert!(matches!(
            deleted,
            ServerEvent::ConversationItemDeleted { item_id } if item_id == "item_1"
        ));

        let item_done: ServerEvent = serde_json::from_str(
            r#"{"type":"response.output_item.done","response_id":"resp_1","output_index":0,"item":{"id":"item_1","type":"message","object":"realtime.item","status":"completed","role":"assistant","content":[{}]}}"#,
        )
        .unwrap();
        assert!(matches!(
            item_done,
            ServerEvent::ResponseOutputItemDone {
                output_index: 0,
                item,
                ..
            } if item.content.len() == 1 && item.content[0].type_.is_none()
        ));

        let limits: ServerEvent = serde_json::from_str(
            r#"{"type":"rate_limits.updated","rate_limits":[{"name":"requests","limit":5,"remaining":4,"reset_seconds":1.0}]}"#,
        )
        .unwrap();
        assert!(matches!(
            limits,
            ServerEvent::RateLimitsUpdated { rate_limits }
                if rate_limits.len() == 1 && rate_limits[0].remaining == 4
        ));
    }

    #[test]
    fn unknown_events_are_forward_compatible_but_known_shapes_are_strict() {
        let event: ServerEvent =
            serde_json::from_str(r#"{"type":"future.event","payload":"ignored"}"#).unwrap();
        assert!(matches!(event, ServerEvent::Unknown));

        assert!(
            serde_json::from_str::<ServerEvent>(
                r#"{"type":"response.text.delta","response_id":"resp_1","delta":"missing item"}"#
            )
            .is_err()
        );
        assert!(
            serde_json::from_str::<ServerEvent>(
                r#"{"type":"error","error":{"message":"missing error type"}}"#
            )
            .is_err()
        );
    }
}
