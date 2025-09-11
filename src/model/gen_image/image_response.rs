use serde::{Deserialize, Serialize};
use validator::Validate;

// Reuse content safety information structure from chat_base_response
use crate::model::chat_base_response::ContentFilterInfo;

/// Image generation response payload
///
/// Fields are optional to be resilient to upstream variations.
#[derive(Clone, Serialize, Deserialize, Validate)]
pub struct ImageResponse {
    /// Request created time, Unix timestamp (seconds)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<u64>,

    /// Array containing generated image URLs. Currently only one image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Vec<ImageDataItem>>,

    /// Content safety related information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_filter: Option<Vec<ContentFilterInfo>>,
}

impl std::fmt::Debug for ImageResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string_pretty(self) {
            Ok(s) => f.write_str(&s),
            Err(_) => f.debug_struct("ImageResponse").finish(),
        }
    }
}

/// Single generated image item
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ImageDataItem {
    /// Image link. Temporary URL valid for ~30 days; persist if needed.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(url)]
    pub url: Option<String>,
}

// --- Getters ---
impl ImageResponse {
    pub fn created(&self) -> Option<u64> {
        self.created
    }
    pub fn data(&self) -> Option<&[ImageDataItem]> {
        self.data.as_deref()
    }
    pub fn content_filter(&self) -> Option<&[ContentFilterInfo]> {
        self.content_filter.as_deref()
    }
}

impl ImageDataItem {
    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }
}
