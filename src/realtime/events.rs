//! Typed events from the official GLM-Realtime protocol.
//!
//! The core wire shapes are pinned to
//! `spec/upstream/asyncapi-2026-07-11.json`; the guide-only conversation,
//! output-item, content-part, and rate-limit events are modeled as well.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::serde_helpers::validate_unique_json;

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
///
/// The realtime session decoder can also preserve an otherwise valid known
/// event containing a future value in a small set of nested protocol enums as
/// [`ServerEvent::UnsupportedKnown`]. Direct Serde deserialization remains
/// strict for those nested values and never constructs that variant.
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

    /// A known event whose otherwise valid payload contains an unsupported
    /// value in a forward-compatible nested protocol enum.
    ///
    /// This variant is constructed only by the realtime session compatibility
    /// decoder. Direct `serde_json` deserialization remains strict. `raw` is
    /// untrusted server data and may contain transcripts, tool arguments, or
    /// other sensitive application content; do not log it without redaction.
    #[serde(skip)]
    #[non_exhaustive]
    UnsupportedKnown {
        /// The known top-level event type from the original payload.
        event_type: String,
        /// The original, unmodified semantic JSON payload.
        raw: Value,
    },

    /// A valid event type introduced by a newer server.
    ///
    /// The event payload is intentionally not retained because it has no
    /// stable schema yet. Applications should use a wildcard match arm and
    /// upgrade the crate when they need the new event.
    #[serde(other)]
    Unknown,
}

/// Deserialize a server event while preserving selected future nested enum
/// values as [`ServerEvent::UnsupportedKnown`].
///
/// Every session-decoded event first receives an allocation-light recursive
/// duplicate-key preflight. The normal typed path then avoids retaining a raw
/// JSON tree. Compatibility probing allocates that tree only after strict
/// typed deserialization fails, and succeeds only when replacing a recognized
/// future enum string makes the complete known event valid.
pub(crate) fn decode_server_event_compat(text: &str) -> serde_json::Result<ServerEvent> {
    validate_unique_json(text)?;

    let strict_error = match serde_json::from_str::<ServerEvent>(text) {
        Ok(event) => return Ok(event),
        Err(error) => error,
    };

    let raw = match serde_json::from_str::<Value>(text) {
        Ok(value) => value,
        Err(_) => return Err(strict_error),
    };
    let Some(event_type) = raw.get("type").and_then(Value::as_str).map(str::to_owned) else {
        return Err(strict_error);
    };

    let mut patched = raw.clone();
    if patch_unsupported_nested_enums(&event_type, &mut patched) == 0 {
        return Err(strict_error);
    }

    match serde_json::from_value::<ServerEvent>(patched) {
        Ok(event) if event_matches_type(&event, &event_type) => {
            Ok(ServerEvent::UnsupportedKnown { event_type, raw })
        },
        _ => Err(strict_error),
    }
}

fn patch_unsupported_nested_enums(event_type: &str, event: &mut Value) -> usize {
    match event_type {
        "session.updated" => patch_session_updated(event),
        "conversation.item.created"
        | "conversation.item.retrieved"
        | "response.output_item.added"
        | "response.output_item.done" => patch_item_type(event),
        _ => 0,
    }
}

fn patch_session_updated(event: &mut Value) -> usize {
    let Some(session) = event.get_mut("session").and_then(Value::as_object_mut) else {
        return 0;
    };
    let mut patched = 0;

    if let Some(modalities) = session.get_mut("modalities").and_then(Value::as_array_mut) {
        for modality in modalities {
            patched += usize::from(replace_unknown_string(modality, &["text", "audio"], "text"));
        }
    }
    if let Some(voice) = session.get_mut("voice") {
        patched += usize::from(replace_unknown_string(
            voice,
            &[
                "tongtong",
                "xiaochen",
                "female-tianmei",
                "male-qn-daxuesheng",
                "male-qn-jingying",
                "lovely_girl",
                "female-shaonv",
            ],
            "tongtong",
        ));
    }
    if let Some(turn_type) = session
        .get_mut("turn_detection")
        .and_then(Value::as_object_mut)
        .and_then(|turn_detection| turn_detection.get_mut("type"))
    {
        patched += usize::from(replace_unknown_string(
            turn_type,
            &["client_vad", "server_vad"],
            "client_vad",
        ));
    }
    if let Some(noise_type) = session
        .get_mut("input_audio_noise_reduction")
        .and_then(Value::as_object_mut)
        .and_then(|noise_reduction| noise_reduction.get_mut("type"))
    {
        patched += usize::from(replace_unknown_string(
            noise_type,
            &["near_field", "far_field"],
            "near_field",
        ));
    }
    if let Some(chat_mode) = session
        .get_mut("beta_fields")
        .and_then(Value::as_object_mut)
        .and_then(|beta_fields| beta_fields.get_mut("chat_mode"))
    {
        patched += usize::from(replace_unknown_string(
            chat_mode,
            &["video_passive", "audio"],
            "audio",
        ));
    }

    // Audio formats deliberately remain fail-closed. RealtimeSession encodes
    // outbound audio using the originally requested input format, and callers
    // interpret output bytes according to the negotiated output format.
    patched
}

fn patch_item_type(event: &mut Value) -> usize {
    event
        .get_mut("item")
        .and_then(Value::as_object_mut)
        .and_then(|item| item.get_mut("type"))
        .map(|item_type| {
            usize::from(replace_unknown_string(
                item_type,
                &["message", "function_call", "function_call_output"],
                "message",
            ))
        })
        .unwrap_or(0)
}

fn replace_unknown_string(value: &mut Value, known: &[&str], replacement: &str) -> bool {
    let Value::String(current) = value else {
        return false;
    };
    if known.contains(&current.as_str()) {
        return false;
    }
    *current = replacement.to_owned();
    true
}

fn event_matches_type(event: &ServerEvent, event_type: &str) -> bool {
    matches!(
        (event_type, event),
        ("session.updated", ServerEvent::SessionUpdated { .. })
            | (
                "conversation.item.created",
                ServerEvent::ConversationItemCreated { .. }
            )
            | (
                "conversation.item.retrieved",
                ServerEvent::ConversationItemRetrieved { .. }
            )
            | (
                "response.output_item.added",
                ServerEvent::ResponseOutputItemAdded { .. }
            )
            | (
                "response.output_item.done",
                ServerEvent::ResponseOutputItemDone { .. }
            )
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::realtime::{InputAudioNoiseReduction, NoiseReductionType, RealtimeVoice};

    fn valid_session_updated() -> Value {
        serde_json::json!({
            "type": "session.updated",
            "session": {
                "input_audio_format": "wav",
                "output_audio_format": "pcm",
                "modalities": ["text", "audio"],
                "voice": "tongtong",
                "turn_detection": { "type": "client_vad" },
                "input_audio_noise_reduction": { "type": "near_field" },
                "beta_fields": { "chat_mode": "audio" }
            }
        })
    }

    fn valid_item_event(event_type: &str) -> Value {
        let item = serde_json::json!({
            "id": "item_1",
            "type": "message",
            "object": "realtime.item",
            "status": "completed",
            "role": "assistant",
            "content": []
        });
        match event_type {
            "conversation.item.created" | "conversation.item.retrieved" => {
                serde_json::json!({ "type": event_type, "item": item })
            },
            "response.output_item.added" | "response.output_item.done" => serde_json::json!({
                "type": event_type,
                "response_id": "resp_1",
                "output_index": 0,
                "item": item
            }),
            _ => panic!("unsupported test event type"),
        }
    }

    fn assert_unsupported(raw: Value, expected_type: &str) {
        let text = serde_json::to_string(&raw).unwrap();
        assert!(
            serde_json::from_str::<ServerEvent>(&text).is_err(),
            "direct Serde unexpectedly accepted {text}"
        );
        match decode_server_event_compat(&text).unwrap() {
            ServerEvent::UnsupportedKnown {
                event_type,
                raw: got,
            } => {
                assert_eq!(event_type, expected_type);
                assert_eq!(got, raw);
            },
            event => panic!("expected UnsupportedKnown, got {event:?}"),
        }
    }

    fn assert_compat_error(raw: Value) {
        let text = serde_json::to_string(&raw).unwrap();
        assert!(
            decode_server_event_compat(&text).is_err(),
            "compatibility decoder unexpectedly accepted {text}"
        );
    }

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
    fn compat_decoder_preserves_future_session_enum_values() {
        for (pointer, future_value) in [
            ("/session/modalities/0", "future_modality"),
            ("/session/voice", "future_voice"),
            ("/session/turn_detection/type", "future_vad"),
            (
                "/session/input_audio_noise_reduction/type",
                "future_noise_reduction",
            ),
            ("/session/beta_fields/chat_mode", "future_chat_mode"),
        ] {
            let mut raw = valid_session_updated();
            *raw.pointer_mut(pointer).unwrap() = Value::String(future_value.to_owned());
            assert_unsupported(raw, "session.updated");
        }
    }

    #[test]
    fn compat_decoder_preserves_future_item_types_for_each_known_event() {
        for event_type in [
            "conversation.item.created",
            "conversation.item.retrieved",
            "response.output_item.added",
            "response.output_item.done",
        ] {
            let mut raw = valid_item_event(event_type);
            *raw.pointer_mut("/item/type").unwrap() = Value::String("future_item".to_owned());
            assert_unsupported(raw, event_type);
        }
    }

    #[test]
    fn compat_decoder_preserves_original_raw_with_multiple_future_values() {
        let mut raw = valid_session_updated();
        *raw.pointer_mut("/session/modalities/0").unwrap() =
            Value::String("future_modality".to_owned());
        *raw.pointer_mut("/session/voice").unwrap() = Value::String("future_voice".to_owned());
        *raw.pointer_mut("/session/turn_detection/type").unwrap() =
            Value::String("future_vad".to_owned());
        *raw.pointer_mut("/session/input_audio_noise_reduction/type")
            .unwrap() = Value::String("future_noise_reduction".to_owned());
        *raw.pointer_mut("/session/beta_fields/chat_mode").unwrap() =
            Value::String("future_chat_mode".to_owned());

        assert_unsupported(raw, "session.updated");
    }

    #[test]
    fn compat_decoder_understands_escaped_candidate_keys_and_values() {
        let text = r#"{"type":"session.updated","sess\u0069on":{"input_audio_format":"wav","output_audio_format":"pcm","vo\u0069ce":"fut\u0075re_voice","turn_detection":{"type":"client_vad"}}}"#;
        assert!(serde_json::from_str::<ServerEvent>(text).is_err());
        match decode_server_event_compat(text).unwrap() {
            ServerEvent::UnsupportedKnown { event_type, raw } => {
                assert_eq!(event_type, "session.updated");
                assert_eq!(raw["session"]["voice"], "future_voice");
            },
            event => panic!("expected UnsupportedKnown, got {event:?}"),
        }
    }

    #[test]
    fn compat_decoder_keeps_audio_format_extensions_fail_closed() {
        for pointer in [
            "/session/input_audio_format",
            "/session/output_audio_format",
        ] {
            let mut raw = valid_session_updated();
            *raw.pointer_mut(pointer).unwrap() = Value::String("future_audio_format".to_owned());
            assert_compat_error(raw);
        }

        let mut raw = valid_session_updated();
        *raw.pointer_mut("/session/input_audio_format").unwrap() =
            Value::String("future_audio_format".to_owned());
        *raw.pointer_mut("/session/voice").unwrap() = Value::String("future_voice".to_owned());
        assert_compat_error(raw);
    }

    #[test]
    fn compat_decoder_does_not_patch_wrong_candidate_types() {
        for (pointer, wrong_value) in [
            ("/session/modalities/0", serde_json::json!(7)),
            ("/session/voice", serde_json::json!({ "future": true })),
            ("/session/turn_detection/type", Value::Null),
            (
                "/session/input_audio_noise_reduction/type",
                serde_json::json!(["near_field"]),
            ),
            ("/session/beta_fields/chat_mode", serde_json::json!(false)),
        ] {
            let mut raw = valid_session_updated();
            *raw.pointer_mut(pointer).unwrap() = wrong_value;
            assert_compat_error(raw);
        }

        let mut raw = valid_item_event("conversation.item.created");
        *raw.pointer_mut("/item/type").unwrap() = serde_json::json!(7);
        assert_compat_error(raw);
    }

    #[test]
    fn compat_decoder_does_not_hide_malformed_sibling_fields() {
        let mut missing_required = valid_session_updated();
        *missing_required.pointer_mut("/session/voice").unwrap() =
            Value::String("future_voice".to_owned());
        missing_required["session"]
            .as_object_mut()
            .unwrap()
            .remove("turn_detection");
        assert_compat_error(missing_required);

        let mut wrong_sibling_type = valid_session_updated();
        *wrong_sibling_type.pointer_mut("/session/voice").unwrap() =
            Value::String("future_voice".to_owned());
        wrong_sibling_type["session"]["temperature"] = Value::String("hot".to_owned());
        assert_compat_error(wrong_sibling_type);

        let mut mixed_modalities = valid_session_updated();
        mixed_modalities["session"]["modalities"] = serde_json::json!(["future_modality", 7]);
        assert_compat_error(mixed_modalities);

        let mut missing_item_object = valid_item_event("conversation.item.created");
        *missing_item_object.pointer_mut("/item/type").unwrap() =
            Value::String("future_item".to_owned());
        missing_item_object["item"]
            .as_object_mut()
            .unwrap()
            .remove("object");
        assert_compat_error(missing_item_object);

        let mut wrong_outer_type = valid_item_event("response.output_item.done");
        *wrong_outer_type.pointer_mut("/item/type").unwrap() =
            Value::String("future_item".to_owned());
        wrong_outer_type["output_index"] = Value::String("zero".to_owned());
        assert_compat_error(wrong_outer_type);
    }

    #[test]
    fn compat_decoder_rejects_duplicate_keys_in_all_session_payloads() {
        let duplicate_payloads = [
            r#"{"type":"session.updated","type":"session.updated","session":{"input_audio_format":"wav","output_audio_format":"pcm","voice":"future_voice","turn_detection":{"type":"client_vad"}}}"#,
            r#"{"type":"session.updated","session":{"input_audio_format":"wav","output_audio_format":"pcm","voice":"tongtong","voice":"future_voice","turn_detection":{"type":"client_vad"}}}"#,
            r#"{"type":"session.updated","session":{"input_audio_format":"wav","input_audio_format":"pcm16","output_audio_format":"pcm","voice":"future_voice","turn_detection":{"type":"client_vad"}}}"#,
            r#"{"type":"session.updated","session":{"input_audio_format":"wav","output_audio_format":"pcm","voice":"future_voice","turn_detection":{"type":"client_vad"},"future":{"nested":1,"nested":2}}}"#,
            r#"{"type":"response.function_call.simple_browser","name":"simple_browser","session":{"role":"safe","role":"admin"}}"#,
            r#"{"type":"future.event","payload":{"role":"safe","role":"admin"}}"#,
        ];

        for text in duplicate_payloads {
            assert!(
                decode_server_event_compat(text).is_err(),
                "duplicate-key payload unexpectedly accepted: {text}"
            );
        }
    }

    #[test]
    fn compat_decoder_preserves_strict_fast_path_behavior() {
        let known = serde_json::to_string(&valid_session_updated()).unwrap();
        assert!(matches!(
            decode_server_event_compat(&known).unwrap(),
            ServerEvent::SessionUpdated { .. }
        ));

        let mut open_fields = valid_item_event("response.output_item.done");
        open_fields["item"]["status"] = Value::String("future_status".to_owned());
        open_fields["item"]["content"] = serde_json::json!([{ "type": "future_content_type" }]);
        let open_fields = serde_json::to_string(&open_fields).unwrap();
        assert!(matches!(
            decode_server_event_compat(&open_fields).unwrap(),
            ServerEvent::ResponseOutputItemDone { .. }
        ));

        assert!(matches!(
            decode_server_event_compat(r#"{"type":"future.event","payload":true}"#).unwrap(),
            ServerEvent::Unknown
        ));
    }

    #[test]
    fn compat_decoder_does_not_search_non_candidate_paths() {
        let text = r#"{"type":"response.text.delta","response_id":"resp_1","delta":"missing item","item":{"type":"future_item","object":"realtime.item"}}"#;
        assert!(decode_server_event_compat(text).is_err());

        let mut raw = valid_session_updated();
        raw["future_extension"] = serde_json::json!({
            "voice": "future_voice",
            "item": { "type": "future_item" }
        });
        let text = serde_json::to_string(&raw).unwrap();
        assert!(matches!(
            decode_server_event_compat(&text).unwrap(),
            ServerEvent::SessionUpdated { .. }
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
