use serde::{Deserialize, Serialize};
use validator::Validate;

/// OCR recognition response
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct OcrResponse {
    /// Task ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,

    /// Message (e.g., success or error description)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    /// Status identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    /// Number of recognition results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub words_result_num: Option<i32>,

    /// Text recognition results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub words_result: Option<Vec<WordsResultItem>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct WordsResultItem {
    /// Location coordinates of the text line
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,

    /// Recognized text content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub words: Option<String>,

    /// Confidence information (only returned when probability=true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub probability: Option<Probability>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Location {
    /// Left coordinate of the rectangle
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left: Option<i32>,

    /// Top coordinate of the rectangle
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top: Option<i32>,

    /// Width of the rectangle
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i32>,

    /// Height of the rectangle
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Probability {
    /// Average confidence
    #[serde(skip_serializing_if = "Option::is_none")]
    pub average: Option<f32>,

    /// Variance of confidence
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variance: Option<f32>,

    /// Minimum confidence
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f32>,
}
