use serde::{Deserialize, Serialize};
use validator::Validate;

/// OCR recognition response
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct OcrResponse {
    /// Task ID
    pub task_id: String,

    /// Message (e.g., success or error description)
    pub message: String,

    /// Status identifier
    pub status: OcrStatus,

    /// Number of recognition results
    pub words_result_num: i64,

    /// Text recognition results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub words_result: Option<Vec<WordsResultItem>>,
}

/// OCR task status defined by the response contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OcrStatus {
    /// Recognition completed successfully.
    Succeeded,
    /// Recognition failed.
    Failed,
}

/// One recognized text line within an [`OcrResponse`].
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct WordsResultItem {
    /// Location coordinates of the text line
    pub location: Location,

    /// Recognized text content
    pub words: String,

    /// Confidence information required by the frozen item schema.
    pub probability: Probability,
}

/// Bounding-box rectangle of a recognized text line.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Location {
    /// Left coordinate of the rectangle
    pub left: i64,

    /// Top coordinate of the rectangle
    pub top: i64,

    /// Width of the rectangle
    pub width: i64,

    /// Height of the rectangle
    pub height: i64,
}

/// Per-character confidence statistics (only returned when `probability=true`).
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Probability {
    /// Average confidence
    pub average: f64,

    /// Variance of confidence
    pub variance: f64,

    /// Minimum confidence
    pub min: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_level_required_fields_and_status_enum_are_enforced() {
        let valid = serde_json::json!({
            "task_id": "ocr-1",
            "message": "ok",
            "status": "succeeded",
            "words_result_num": 0
        });
        assert!(serde_json::from_value::<OcrResponse>(valid).is_ok());
        assert!(serde_json::from_str::<OcrResponse>("{}").is_err());
        assert!(
            serde_json::from_value::<OcrResponse>(serde_json::json!({
                "task_id": "ocr-1",
                "message": "ok",
                "status": "SUCCESS",
                "words_result_num": 0
            }))
            .is_err()
        );
    }

    #[test]
    fn result_item_enforces_nested_required_fields() {
        assert!(serde_json::from_str::<WordsResultItem>(r#"{"words":"hello"}"#).is_err());
        let item = serde_json::json!({
            "location": {"left": 1, "top": 2, "width": 3, "height": 4},
            "words": "hello",
            "probability": {"average": 0.9, "variance": 0.1, "min": 0.7}
        });
        assert!(serde_json::from_value::<WordsResultItem>(item).is_ok());
    }
}
