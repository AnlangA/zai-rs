use serde::Serialize;
use validator::Validate;

/// Request body for voice deletion.
#[derive(Clone, Serialize, Validate)]
#[validate(schema(function = "validate_voice_delete_body"))]
pub struct VoiceDeleteBody {
    /// Identifier of the voice to delete.
    #[validate(length(min = 1))]
    pub voice: String,

    /// Optional client-provided request identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl std::fmt::Debug for VoiceDeleteBody {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("VoiceDeleteBody")
            .field("voice", &"[REDACTED]")
            .field(
                "request_id",
                &self.request_id.as_ref().map(|_| "[REDACTED]"),
            )
            .finish()
    }
}

fn validate_voice_delete_body(body: &VoiceDeleteBody) -> Result<(), validator::ValidationError> {
    if body.voice.trim().is_empty() {
        Err(validator::ValidationError::new("voice_must_not_be_blank"))
    } else {
        Ok(())
    }
}

impl VoiceDeleteBody {
    /// Create a new voice-delete body for the given voice id.
    pub fn new(voice: impl Into<String>) -> Self {
        Self {
            voice: voice.into(),
            request_id: None,
        }
    }
    /// Set the optional client-provided request id.
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_redacts_voice_and_request_identifiers() {
        let body = VoiceDeleteBody::new("private-voice").with_request_id("private-request");
        let debug = format!("{body:?}");
        assert!(!debug.contains("private-voice"));
        assert!(!debug.contains("private-request"));
    }
}
