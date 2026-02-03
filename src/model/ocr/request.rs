use serde::Serialize;
use validator::Validate;

/// OCR tool types
#[derive(Debug, Clone, Serialize)]
pub enum OcrToolType {
    #[serde(rename = "hand_write")]
    HandWrite,
}

/// Language types for OCR recognition
#[derive(Debug, Clone, Serialize)]
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

/// Body parameters holder for OCR request (used to build multipart form)
#[derive(Debug, Clone, Serialize, Validate)]
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
    pub request_id: Option<String>,

    /// End user id (6..=128 chars)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 6, max = 128))]
    pub user_id: Option<String>,
}

impl Default for OcrBody {
    fn default() -> Self {
        Self::new()
    }
}

impl OcrBody {
    pub fn new() -> Self {
        Self {
            tool_type: None,
            language_type: None,
            probability: None,
            request_id: None,
            user_id: None,
        }
    }

    pub fn with_tool_type(mut self, tool_type: OcrToolType) -> Self {
        self.tool_type = Some(tool_type);
        self
    }

    pub fn with_language_type(mut self, language_type: OcrLanguageType) -> Self {
        self.language_type = Some(language_type);
        self
    }

    pub fn with_probability(mut self, probability: bool) -> Self {
        self.probability = Some(probability);
        self
    }

    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }
}
