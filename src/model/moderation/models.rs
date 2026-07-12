//! Content-moderation wire types.

use serde::{Deserialize, Deserializer, Serialize, de::Error as _, ser::SerializeStruct};
use validator::Validate;

/// Content moderation model type.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ModerationModel {
    /// Current moderation model.
    #[serde(rename = "moderation")]
    #[default]
    Moderation,
}

/// Moderation input content.
#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ModerationInput {
    /// Text content.
    Text(String),
    /// Multimedia content with its type and URL.
    Multimedia(MultimediaInput),
    /// Multiple structured text and multimedia items.
    Items(Vec<ModerationItem>),
}

impl std::fmt::Debug for ModerationInput {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text(_) => formatter.debug_tuple("Text").field(&"[REDACTED]").finish(),
            Self::Multimedia(value) => formatter.debug_tuple("Multimedia").field(value).finish(),
            Self::Items(values) => formatter
                .debug_struct("Items")
                .field("len", &values.len())
                .finish(),
        }
    }
}

/// Multimedia input for content moderation.
#[derive(Clone, Validate)]
pub struct MultimediaInput {
    /// Content type.
    pub content_type: MediaType,
    /// URL of the multimedia content.
    #[validate(url)]
    pub url: String,
}

impl std::fmt::Debug for MultimediaInput {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MultimediaInput")
            .field("content_type", &self.content_type)
            .field("url", &"[REDACTED]")
            .finish()
    }
}

#[derive(Serialize, Deserialize)]
struct UrlValue<T> {
    url: T,
}

impl Serialize for MultimediaInput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("MultimediaInput", 2)?;
        state.serialize_field("type", &self.content_type)?;
        let value = UrlValue { url: &self.url };
        state.serialize_field(self.content_type.content_key(), &value)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for MultimediaInput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct WireInput {
            #[serde(rename = "type")]
            content_type: MediaType,
            image_url: Option<UrlValue<String>>,
            audio_url: Option<UrlValue<String>>,
            video_url: Option<UrlValue<String>>,
        }

        let wire = WireInput::deserialize(deserializer)?;
        let value = match wire.content_type {
            MediaType::Image => wire.image_url,
            MediaType::Audio => wire.audio_url,
            MediaType::Video => wire.video_url,
        }
        .ok_or_else(|| {
            serde::de::Error::custom(format_args!(
                "missing {} object for moderation input",
                wire.content_type.content_key()
            ))
        })?;

        Ok(Self {
            content_type: wire.content_type,
            url: value.url,
        })
    }
}

/// Media types supported for moderation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaType {
    /// Image content.
    #[serde(rename = "image_url")]
    Image,
    /// Audio content.
    #[serde(rename = "audio_url")]
    Audio,
    /// Video content.
    #[serde(rename = "video_url")]
    Video,
}

impl MediaType {
    const fn content_key(self) -> &'static str {
        match self {
            Self::Image => "image_url",
            Self::Audio => "audio_url",
            Self::Video => "video_url",
        }
    }
}

/// One structured item in a batch moderation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ModerationItem {
    /// Text item encoded as `{ "type": "text", "text": "..." }`.
    Text(ModerationTextItem),
    /// Image, audio, or video item.
    Multimedia(MultimediaInput),
}

/// Structured text input used inside a batch moderation request.
#[derive(Clone, Serialize, Deserialize)]
pub struct ModerationTextItem {
    #[serde(rename = "type")]
    content_type: TextMediaType,
    /// Text to moderate.
    pub text: String,
}

impl std::fmt::Debug for ModerationTextItem {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ModerationTextItem")
            .field("content_type", &self.content_type)
            .field("text", &"[REDACTED]")
            .finish()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum TextMediaType {
    #[serde(rename = "text")]
    Text,
}

impl ModerationItem {
    /// Create a structured text item.
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text(ModerationTextItem {
            content_type: TextMediaType::Text,
            text: text.into(),
        })
    }

    /// Create a structured multimedia item.
    pub fn multimedia(content_type: MediaType, url: impl Into<String>) -> Self {
        Self::Multimedia(MultimediaInput {
            content_type,
            url: url.into(),
        })
    }
}

/// Content moderation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationRequest {
    /// Moderation model.
    #[serde(default)]
    pub model: ModerationModel,
    /// Content to moderate.
    pub input: ModerationInput,
}

impl ModerationRequest {
    /// Create a new moderation request with text content.
    ///
    /// The service accepts at most 2,000 Unicode characters.
    pub fn new_text(text: impl Into<String>) -> Self {
        Self {
            model: ModerationModel::default(),
            input: ModerationInput::Text(text.into()),
        }
    }

    /// Create a new moderation request with multimedia content.
    pub fn new_multimedia(content_type: MediaType, url: impl Into<String>) -> Self {
        Self {
            model: ModerationModel::default(),
            input: ModerationInput::Multimedia(MultimediaInput {
                content_type,
                url: url.into(),
            }),
        }
    }

    /// Create a request containing multiple structured items.
    pub fn new_items(items: Vec<ModerationItem>) -> Self {
        Self {
            model: ModerationModel::default(),
            input: ModerationInput::Items(items),
        }
    }

    /// Validate request constraints before dispatch.
    pub fn validate(&self) -> Result<(), validator::ValidationErrors> {
        let mut errors = validator::ValidationErrors::new();

        match &self.input {
            ModerationInput::Text(text) => validate_text(text, &mut errors),
            ModerationInput::Multimedia(multimedia) => {
                validate_multimedia(multimedia, &mut errors);
            },
            ModerationInput::Items(items) if items.is_empty() => {
                errors.add("input", validator::ValidationError::new("items_required"));
            },
            ModerationInput::Items(items) => {
                for item in items {
                    match item {
                        ModerationItem::Text(item) => validate_text(&item.text, &mut errors),
                        ModerationItem::Multimedia(item) => {
                            validate_multimedia(item, &mut errors);
                        },
                    }
                }
            },
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod request_debug_tests {
    use super::*;

    #[test]
    fn request_debug_redacts_text_and_media_urls() {
        let text = format!(
            "{:?}",
            ModerationRequest::new_text("private moderation text")
        );
        assert!(!text.contains("private moderation text"));

        let media = format!(
            "{:?}",
            ModerationRequest::new_multimedia(
                MediaType::Image,
                "https://private.example/image.png"
            )
        );
        assert!(!media.contains("private.example"));

        let items = format!(
            "{:?}",
            ModerationRequest::new_items(vec![ModerationItem::text("private item")])
        );
        assert!(!items.contains("private item"));
        assert!(items.contains("len: 1"));
    }
}

fn validate_text(text: &str, errors: &mut validator::ValidationErrors) {
    if text.trim().is_empty() {
        errors.add("input", validator::ValidationError::new("text_required"));
    } else if text.chars().count() > 2000 {
        errors.add(
            "input",
            validator::ValidationError::new("text_length_exceeded"),
        );
    }
}

fn validate_multimedia(multimedia: &MultimediaInput, errors: &mut validator::ValidationErrors) {
    let valid_url = multimedia
        .url
        .parse::<url::Url>()
        .is_ok_and(|url| matches!(url.scheme(), "http" | "https"));
    if !valid_url {
        errors.add("input", validator::ValidationError::new("invalid_url"));
    }
}

/// Risk level for moderated content.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub enum RiskLevel {
    /// No risk detected.
    #[default]
    #[serde(rename = "PASS")]
    Pass,
    /// Suspicious content that requires review.
    #[serde(rename = "REVIEW")]
    Review,
    /// Policy-violating content that should be rejected.
    #[serde(rename = "REJECT")]
    Reject,
    /// A value introduced by a newer service revision.
    #[serde(other)]
    Unknown,
}

/// Moderation result for a single content item.
///
/// The frozen OpenAPI schema does not mark any item property as required.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationResult {
    /// Type of content that was moderated.
    #[serde(rename = "content_type", skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    /// Assessed risk level.
    #[serde(rename = "risk_level", skip_serializing_if = "Option::is_none")]
    pub risk_level: Option<RiskLevel>,
    /// Detected risk types.
    #[serde(rename = "risk_type", skip_serializing_if = "Option::is_none")]
    pub risk_types: Option<Vec<String>>,
}

/// Usage statistics for moderation API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationUsage {
    /// Text-moderation usage statistics.
    #[serde(rename = "moderation_text", skip_serializing_if = "Option::is_none")]
    pub moderation_text: Option<ModerationTextUsage>,
}

/// Text moderation usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationTextUsage {
    /// Number of text-moderation calls.
    #[serde(rename = "call_count", skip_serializing_if = "Option::is_none")]
    pub call_count: Option<f64>,
}

/// Content moderation response.
#[derive(Debug, Clone, Serialize)]
pub struct ModerationResponse {
    /// Task identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Request creation time as a Unix timestamp in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<u64>,
    /// Request identifier.
    #[serde(
        rename = "request_id",
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::super::serde_helpers::optional_string_from_number_or_string"
    )]
    pub request_id: Option<String>,
    /// Moderation results.
    #[serde(rename = "result_list", skip_serializing_if = "Option::is_none")]
    pub result_list: Option<Vec<ModerationResult>>,
    /// Usage statistics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<ModerationUsage>,
}

#[derive(Deserialize)]
struct ModerationResponseWire {
    id: Option<String>,
    created: Option<u64>,
    #[serde(
        default,
        deserialize_with = "super::super::serde_helpers::optional_string_from_number_or_string"
    )]
    request_id: Option<String>,
    result_list: Option<Vec<ModerationResult>>,
    usage: Option<ModerationUsage>,
}

impl<'de> Deserialize<'de> for ModerationResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ModerationResponseWire::deserialize(deserializer)?;
        if wire.id.is_none()
            && wire.created.is_none()
            && wire.request_id.is_none()
            && wire.result_list.is_none()
            && wire.usage.is_none()
        {
            return Err(D::Error::custom(
                "moderation response contained no documented fields",
            ));
        }
        Ok(Self {
            id: wire.id,
            created: wire.created,
            request_id: wire.request_id,
            result_list: wire.result_list,
            usage: wire.usage,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multimedia_uses_the_nested_wire_shape() {
        let request =
            ModerationRequest::new_multimedia(MediaType::Image, "https://example.com/image.png");
        let value = serde_json::to_value(request).unwrap();
        assert_eq!(value["input"]["type"], "image_url");
        assert_eq!(
            value["input"]["image_url"]["url"],
            "https://example.com/image.png"
        );
    }

    #[test]
    fn batch_items_use_structured_text_and_media_shapes() {
        let request = ModerationRequest::new_items(vec![
            ModerationItem::text("hello"),
            ModerationItem::multimedia(MediaType::Audio, "https://example.com/audio.mp3"),
        ]);
        assert!(request.validate().is_ok());
        let value = serde_json::to_value(request).unwrap();
        assert_eq!(value["input"][0]["type"], "text");
        assert_eq!(value["input"][1]["type"], "audio_url");
        assert_eq!(
            value["input"][1]["audio_url"]["url"],
            "https://example.com/audio.mp3"
        );
    }

    #[test]
    fn text_limit_counts_characters_instead_of_utf8_bytes() {
        assert!(ModerationRequest::new_text(" ").validate().is_err());
        assert!(
            ModerationRequest::new_text("测".repeat(2_000))
                .validate()
                .is_ok()
        );
        assert!(
            ModerationRequest::new_text("测".repeat(2_001))
                .validate()
                .is_err()
        );
    }

    #[test]
    fn response_requires_one_documented_non_null_field() {
        assert!(serde_json::from_str::<ModerationResponse>("{}").is_err());
        assert!(serde_json::from_str::<ModerationResponse>(r#"{"id":null}"#).is_err());
        let response: ModerationResponse = serde_json::from_str(r#"{"id":"mod-1"}"#).unwrap();
        assert!(response.request_id.is_none());
    }

    #[test]
    fn result_fields_are_optional_without_defaulting_risk_to_pass() {
        let result: ModerationResult = serde_json::from_str("{}").unwrap();
        assert!(result.content_type.is_none());
        assert!(result.risk_level.is_none());
        assert!(result.risk_types.is_none());
    }
}
