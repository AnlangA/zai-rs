use serde::{Deserialize, Serialize};
use validator::Validate;

/// ASR transcription response
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct AudioTranscriptionResponse {
    /// Task ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Request created time, Unix timestamp (seconds)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<u64>,

    /// Client-provided request id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,

    /// Model name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Segmented ASR content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segments: Option<Vec<SegmentItem>>,

    /// Full transcription text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SegmentItem {
    /// Segment index
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,
    /// Segment start time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<f32>,
    /// Segment end time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<f32>,
    /// Segment text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}
