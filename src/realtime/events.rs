//! Typed realtime events, mapped 1:1 to the official GLM-Realtime protocol
//! client/server event names.
//!
//! See <https://github.com/MetaGLM/glm-realtime-sdk/blob/main/GLM-Realtime-doc-for-llm.md>.

use serde::{Deserialize, Serialize};

use super::protocol::{RealtimeConversationItem, RealtimeResponse, SessionConfig};

/// Body of a server `error` event.
#[derive(Debug, Clone, Deserialize)]
pub struct ServerErrorBody {
    /// Error type, e.g. `"invalid_request_error"`, `"server_error"`.
    #[serde(rename = "type", default)]
    pub type_: Option<String>,
    /// Machine-readable error code (string per the GLM protocol).
    #[serde(default)]
    pub code: Option<String>,
    /// Human-readable message.
    #[serde(default)]
    pub message: Option<String>,
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
        #[serde(skip_serializing_if = "Option::is_none")]
        event_id: Option<String>,
        session: SessionConfig,
    },

    /// `input_audio_buffer.append` — upload base64 WAV audio.
    #[serde(rename = "input_audio_buffer.append")]
    InputAudioBufferAppend {
        audio: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        client_timestamp: Option<i64>,
    },

    /// `input_audio_buffer.append_video_frame` — upload a base64 JPEG frame.
    #[serde(rename = "input_audio_buffer.append_video_frame")]
    InputAudioBufferAppendVideoFrame {
        video_frame: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        client_timestamp: Option<i64>,
    },

    /// `input_audio_buffer.commit` — commit buffered audio for inference.
    #[serde(rename = "input_audio_buffer.commit")]
    InputAudioBufferCommit {
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
        #[serde(skip_serializing_if = "Option::is_none")]
        event_id: Option<String>,
        item: RealtimeConversationItem,
    },

    /// `response.create` — trigger model inference.
    #[serde(rename = "response.create")]
    ResponseCreate {
        #[serde(skip_serializing_if = "Option::is_none")]
        client_timestamp: Option<i64>,
    },

    /// `response.cancel` — cancel the in-flight response (interruption).
    #[serde(rename = "response.cancel")]
    ResponseCancel {
        #[serde(skip_serializing_if = "Option::is_none")]
        client_timestamp: Option<i64>,
    },
}

// ---------------------------------------------------------------------------
// Server events (received server → client). Deserialize-only.
// ---------------------------------------------------------------------------

/// A server event, tagged by `type`. Unknown/extra fields are ignored.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ServerEvent {
    /// Server-side error (most are recoverable; the session stays open).
    #[serde(rename = "error")]
    Error { error: ServerErrorBody },

    /// `session.created` — session established.
    #[serde(rename = "session.created")]
    SessionCreated,

    /// `session.updated` — confirms a `session.update`.
    #[serde(rename = "session.updated")]
    SessionUpdated,

    /// `conversation.created` — one per session.
    #[serde(rename = "conversation.created")]
    ConversationCreated,

    /// `conversation.item.created`.
    #[serde(rename = "conversation.item.created")]
    ConversationItemCreated { item: RealtimeConversationItem },

    /// `conversation.item.input_audio_transcription.completed`.
    #[serde(rename = "conversation.item.input_audio_transcription.completed")]
    InputAudioTranscriptionCompleted { item_id: String, transcript: String },

    /// `conversation.item.input_audio_transcription.failed`.
    #[serde(rename = "conversation.item.input_audio_transcription.failed")]
    InputAudioTranscriptionFailed {
        item_id: String,
        error: ServerErrorBody,
    },

    /// `input_audio_buffer.committed`.
    #[serde(rename = "input_audio_buffer.committed")]
    InputAudioBufferCommitted {
        #[serde(default)]
        item_id: Option<String>,
    },

    /// `input_audio_buffer.cleared`.
    #[serde(rename = "input_audio_buffer.cleared")]
    InputAudioBufferCleared,

    /// `input_audio_buffer.speech_started` (server-VAD only).
    #[serde(rename = "input_audio_buffer.speech_started")]
    InputAudioBufferSpeechStarted,

    /// `input_audio_buffer.speech_stopped` (server-VAD only).
    #[serde(rename = "input_audio_buffer.speech_stopped")]
    InputAudioBufferSpeechStopped,

    /// `response.created`.
    #[serde(rename = "response.created")]
    ResponseCreated { response: RealtimeResponse },

    /// `response.done` — final state + usage. Always emitted.
    #[serde(rename = "response.done")]
    ResponseDone { response: RealtimeResponse },

    /// `response.audio.delta` — base64 audio chunk (mp3 or pcm).
    #[serde(rename = "response.audio.delta")]
    ResponseAudioDelta {
        response_id: String,
        #[serde(default)]
        item_id: Option<String>,
        delta: String,
    },

    /// `response.audio.done`.
    #[serde(rename = "response.audio.done")]
    ResponseAudioDone {
        response_id: String,
        #[serde(default)]
        item_id: Option<String>,
    },

    /// `response.audio_transcript.delta` — incremental transcript text.
    #[serde(rename = "response.audio_transcript.delta")]
    ResponseAudioTranscriptDelta { response_id: String, delta: String },

    /// `response.audio_transcript.done` — final transcript.
    #[serde(rename = "response.audio_transcript.done")]
    ResponseAudioTranscriptDone {
        response_id: String,
        transcript: String,
    },

    /// `response.function_call_arguments.done` — completed tool call.
    #[serde(rename = "response.function_call_arguments.done")]
    ResponseFunctionCallArgumentsDone {
        response_id: String,
        name: String,
        arguments: String,
    },

    /// `response.function_call.simple_browser` — video link triggered search.
    #[serde(rename = "response.function_call.simple_browser")]
    ResponseFunctionCallSimpleBrowser,

    /// `heartbeat` — keepalive (every ~30s).
    #[serde(rename = "heartbeat")]
    Heartbeat,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_update_serializes_to_official_shape() {
        let ev = ClientEvent::SessionUpdate {
            event_id: Some("evt_1".into()),
            session: SessionConfig::default(),
        };
        let json = serde_json::to_value(&ev).unwrap();
        assert_eq!(json["type"], "session.update");
        assert_eq!(json["event_id"], "evt_1");
        assert_eq!(json["session"]["input_audio_format"], "wav");
        assert_eq!(json["session"]["output_audio_format"], "pcm");
        assert_eq!(json["session"]["turn_detection"]["type"], "client_vad");
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
    fn server_audio_delta_parses_official_example() {
        // Trimmed shape of the official response.audio.delta example.
        let raw = r#"{"event_id":"event89","type":"response.audio.delta","client_timestamp":1737454096061,"response_id":"respbc50304acdea479b8bd55efd5346dbdf","output_index":0,"content_index":0,"delta":"+w6hBu39"}"#;
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
                assert_eq!(response.status.as_deref(), Some("completed"));
                let usage = response.usage.unwrap();
                assert_eq!(usage.total_tokens, 7);
                assert_eq!(usage.output_tokens, 3);
            },
            _ => panic!("wrong variant"),
        }
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
}
