use serde::{Deserialize, Serialize};

/// Complete non-streaming ASR transcription response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioToTextResponse {
    /// Task ID.
    pub id: String,

    /// Request created time, Unix timestamp (seconds)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<i64>,

    /// Client-provided request id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,

    /// Model name.
    pub model: String,

    /// Full transcription text.
    pub text: String,
}

/// One typed event from a streaming ASR transcription.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeechToTextEvent {
    /// Task ID, when supplied by this event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Request creation time as Unix seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<i64>,
    /// Model name, when supplied by this event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Upstream event type, such as `transcript.text.delta` or
    /// `transcript.text.done`. Kept open for forward compatibility.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub event_type: Option<String>,
    /// Incremental transcription text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nonstream_response_requires_id_model_and_text() {
        for missing in ["id", "model", "text"] {
            let mut value = serde_json::json!({
                "id": "asr-1",
                "model": "glm-asr-2512",
                "text": "hello"
            });
            value.as_object_mut().unwrap().remove(missing);
            assert!(serde_json::from_value::<AudioToTextResponse>(value).is_err());
        }
    }

    #[test]
    fn stream_event_preserves_open_event_type() {
        let event: SpeechToTextEvent = serde_json::from_value(serde_json::json!({
            "id": "asr-1",
            "type": "transcript.text.future",
            "delta": "hi"
        }))
        .unwrap();
        assert_eq!(event.event_type.as_deref(), Some("transcript.text.future"));
        assert_eq!(event.delta.as_deref(), Some("hi"));
    }
}
