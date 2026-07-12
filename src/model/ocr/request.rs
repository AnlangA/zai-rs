use serde::Serialize;
use validator::Validate;

/// OCR tool types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum OcrToolType {
    /// Handwriting recognition.
    #[serde(rename = "hand_write")]
    HandWrite,
}

impl OcrToolType {
    /// Return the exact multipart form value expected by the OCR endpoint.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::HandWrite => "hand_write",
        }
    }
}

/// Language types for OCR recognition
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum OcrLanguageType {
    /// Auto-detect language
    #[serde(rename = "AUTO")]
    Auto,

    /// Chinese and English (default)
    #[serde(rename = "CHN_ENG")]
    ChnEng,

    /// English
    #[serde(rename = "ENG")]
    Eng,

    /// Japanese
    #[serde(rename = "JAP")]
    Jap,

    /// Korean
    #[serde(rename = "KOR")]
    Kor,

    /// French
    #[serde(rename = "FRE")]
    Fre,

    /// Spanish
    #[serde(rename = "SPA")]
    Spa,

    /// Portuguese
    #[serde(rename = "POR")]
    Por,

    /// German
    #[serde(rename = "GER")]
    Ger,

    /// Italian
    #[serde(rename = "ITA")]
    Ita,

    /// Russian
    #[serde(rename = "RUS")]
    Rus,

    /// Danish
    #[serde(rename = "DAN")]
    Dan,

    /// Dutch
    #[serde(rename = "DUT")]
    Dut,

    /// Malay
    #[serde(rename = "MAL")]
    Mal,

    /// Swedish
    #[serde(rename = "SWE")]
    Swe,

    /// Indonesian
    #[serde(rename = "IND")]
    Ind,

    /// Polish
    #[serde(rename = "POL")]
    Pol,

    /// Romanian
    #[serde(rename = "ROM")]
    Rom,

    /// Turkish
    #[serde(rename = "TUR")]
    Tur,

    /// Greek
    #[serde(rename = "GRE")]
    Gre,

    /// Hungarian
    #[serde(rename = "HUN")]
    Hun,

    /// Thai
    #[serde(rename = "THA")]
    Tha,

    /// Vietnamese
    #[serde(rename = "VIE")]
    Vie,

    /// Arabic
    #[serde(rename = "ARA")]
    Ara,

    /// Hindi
    #[serde(rename = "HIN")]
    Hin,
}

impl OcrLanguageType {
    /// Return the exact multipart form value expected by the OCR endpoint.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "AUTO",
            Self::ChnEng => "CHN_ENG",
            Self::Eng => "ENG",
            Self::Jap => "JAP",
            Self::Kor => "KOR",
            Self::Fre => "FRE",
            Self::Spa => "SPA",
            Self::Por => "POR",
            Self::Ger => "GER",
            Self::Ita => "ITA",
            Self::Rus => "RUS",
            Self::Dan => "DAN",
            Self::Dut => "DUT",
            Self::Mal => "MAL",
            Self::Swe => "SWE",
            Self::Ind => "IND",
            Self::Pol => "POL",
            Self::Rom => "ROM",
            Self::Tur => "TUR",
            Self::Gre => "GRE",
            Self::Hun => "HUN",
            Self::Tha => "THA",
            Self::Vie => "VIE",
            Self::Ara => "ARA",
            Self::Hin => "HIN",
        }
    }
}

/// Body parameters holder for OCR request (used to build multipart form)
#[derive(Clone, Serialize, Validate)]
pub struct OcrBody {
    /// Tool type (fixed as "hand_write" for handwriting recognition)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_type: Option<OcrToolType>,

    /// Language type for recognition (default: CHN_ENG)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_type: Option<OcrLanguageType>,

    /// Whether to return confidence information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub probability: Option<bool>,

    /// Client-provided unique request id
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 6, max = 64))]
    pub request_id: Option<String>,

    /// End user id (6..=128 chars)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 6, max = 128))]
    pub user_id: Option<String>,
}

impl std::fmt::Debug for OcrBody {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OcrBody")
            .field("tool_type", &self.tool_type)
            .field("language_type", &self.language_type)
            .field("probability", &self.probability)
            .field(
                "request_id",
                &self.request_id.as_ref().map(|_| "[REDACTED]"),
            )
            .field("user_id", &self.user_id.as_ref().map(|_| "[REDACTED]"))
            .finish()
    }
}

impl Default for OcrBody {
    fn default() -> Self {
        Self::new()
    }
}

impl OcrBody {
    /// Create a new empty OCR body (all fields `None`).
    pub fn new() -> Self {
        Self {
            tool_type: None,
            language_type: None,
            probability: None,
            request_id: None,
            user_id: None,
        }
    }

    /// Set the OCR tool type.
    pub fn with_tool_type(mut self, tool_type: OcrToolType) -> Self {
        self.tool_type = Some(tool_type);
        self
    }

    /// Set the document language.
    pub fn with_language_type(mut self, language_type: OcrLanguageType) -> Self {
        self.language_type = Some(language_type);
        self
    }

    /// Whether to return per-character recognition probabilities.
    pub fn with_probability(mut self, probability: bool) -> Self {
        self.probability = Some(probability);
        self
    }

    /// Set the client-side request id.
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    /// Set the end-user id (6..=128 chars).
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_redacts_request_and_user_identifiers() {
        let body = OcrBody::new()
            .with_request_id("private-request")
            .with_user_id("private-user");
        let debug = format!("{body:?}");
        assert!(!debug.contains("private-request"));
        assert!(!debug.contains("private-user"));
    }
}
