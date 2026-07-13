//! Typed requests for assistant invocation and discovery endpoints.

use serde::{Deserialize, Serialize};

use crate::{ZaiResult, client::ZaiClient};

use super::response::{
    AssistantConversationListResponse, AssistantInvokeResponse, AssistantListResponse,
};

/// Assistant identifiers accepted by the frozen assistant API schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AssistantId {
    /// Official ChatGLM assistant.
    #[serde(rename = "65940acff94777010aa6b796")]
    ChatGlm,
    /// Official data-analysis assistant.
    #[serde(rename = "65a265419d72d299a9230616")]
    DataAnalysis,
    /// Official complex-flowchart assistant.
    #[serde(rename = "664dd7bd5bb3a13ba0f81668")]
    Flowchart,
    /// Official mind-map assistant.
    #[serde(rename = "664e0cade018d633146de0d2")]
    MindMap,
    /// Official prompt-engineering assistant.
    #[serde(rename = "6654898292788e88ce9e7f4c")]
    PromptEngineer,
    /// Official image-generation assistant.
    #[serde(rename = "66437ef3d920bdc5c60f338e")]
    ImageGeneration,
    /// Official AI-search assistant.
    #[serde(rename = "659e54b1b8006379b4b2abd6")]
    AiSearch,
    /// Official presentation assistant.
    #[serde(rename = "65d2f07bb2c10188f885bd89")]
    Presentation,
    /// Official arXiv paper-reading assistant.
    #[serde(rename = "663058948bb259b7e8a22730")]
    ArxivReader,
    /// Official programmer assistant Sam.
    #[serde(rename = "65a393b3619c6f13586246cd")]
    ProgrammerSam,
    /// Official web-novel writing assistant.
    #[serde(rename = "65b356af6924a59d52832e54")]
    WebNovelWriter,
    /// Official English-grammar assistant.
    #[serde(rename = "668fdd45405f2e3c9f71f832")]
    EnglishGrammar,
}

/// Model accepted by the assistant invocation endpoint.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AssistantModel {
    /// Standard assistant model.
    #[default]
    #[serde(rename = "glm-4-assistant")]
    Glm4Assistant,
    /// Assistant model with all-tools support.
    #[serde(rename = "glm-4-alltools")]
    Glm4AllTools,
}

/// Role accepted by an assistant request message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AssistantMessageRole {
    /// User-authored input (the only role in the frozen schema).
    #[serde(rename = "user")]
    User,
}

/// Type of a multimodal assistant request part.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AssistantContentType {
    /// Text input.
    #[serde(rename = "text")]
    Text,
    /// Image URL input.
    #[serde(rename = "image_url")]
    ImageUrl,
}

/// URL wrapper used by an image request part.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssistantImageUrl {
    /// Image URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl std::fmt::Debug for AssistantImageUrl {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AssistantImageUrl")
            .field("url_configured", &self.url.is_some())
            .finish()
    }
}

impl AssistantImageUrl {
    /// Create an image URL wrapper.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: Some(url.into()),
        }
    }
}

/// One part of a multimodal assistant request message.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssistantContentPart {
    /// Part type.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<AssistantContentType>,
    /// Text input for a text part.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// URL input for an image part.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<AssistantImageUrl>,
}

impl std::fmt::Debug for AssistantContentPart {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AssistantContentPart")
            .field("type", &self.type_)
            .field("text_configured", &self.text.is_some())
            .field("image_url_configured", &self.image_url.is_some())
            .finish()
    }
}

impl AssistantContentPart {
    /// Create a text part.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            type_: Some(AssistantContentType::Text),
            text: Some(text.into()),
            image_url: None,
        }
    }

    /// Create an image URL part.
    pub fn image_url(url: impl Into<String>) -> Self {
        Self {
            type_: Some(AssistantContentType::ImageUrl),
            text: None,
            image_url: Some(AssistantImageUrl::new(url)),
        }
    }

    fn validate(&self) -> ZaiResult<()> {
        match self.type_ {
            Some(AssistantContentType::Text) => {
                let Some(text) = self.text.as_deref() else {
                    return Err(crate::client::validation::invalid(
                        "assistant text part requires text",
                    ));
                };
                crate::client::validation::require_non_blank(text, "assistant text")?;
                if self.image_url.is_some() {
                    return Err(crate::client::validation::invalid(
                        "assistant text part must not contain image_url",
                    ));
                }
            },
            Some(AssistantContentType::ImageUrl) => {
                let Some(url) = self
                    .image_url
                    .as_ref()
                    .and_then(|image| image.url.as_deref())
                else {
                    return Err(crate::client::validation::invalid(
                        "assistant image part requires image_url.url",
                    ));
                };
                crate::client::validation::require_non_blank(url, "assistant image URL")?;
                if self.text.is_some() {
                    return Err(crate::client::validation::invalid(
                        "assistant image part must not contain text",
                    ));
                }
            },
            None => {
                return Err(crate::client::validation::invalid(
                    "assistant content part requires type",
                ));
            },
        }
        Ok(())
    }
}

/// Plain text or multimodal content in an assistant request message.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AssistantMessageContent {
    /// Plain text input.
    Text(String),
    /// Multimodal input parts.
    Parts(Vec<AssistantContentPart>),
}

impl std::fmt::Debug for AssistantMessageContent {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text(_) => formatter.write_str("Text([REDACTED])"),
            Self::Parts(parts) => formatter
                .debug_struct("Parts")
                .field("count", &parts.len())
                .finish(),
        }
    }
}

impl AssistantMessageContent {
    fn validate(&self) -> ZaiResult<()> {
        match self {
            Self::Text(text) => {
                crate::client::validation::require_non_blank(text, "assistant message text")
            },
            Self::Parts(parts) => {
                if parts.is_empty() {
                    return Err(crate::client::validation::invalid(
                        "assistant content parts must not be empty",
                    ));
                }
                parts.iter().try_for_each(AssistantContentPart::validate)
            },
        }
    }
}

impl From<String> for AssistantMessageContent {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<&str> for AssistantMessageContent {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

impl From<Vec<AssistantContentPart>> for AssistantMessageContent {
    fn from(value: Vec<AssistantContentPart>) -> Self {
        Self::Parts(value)
    }
}

/// One user message sent to an assistant.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssistantMessage {
    /// Message role (`user`).
    pub role: AssistantMessageRole,
    /// Plain text or multimodal message content.
    pub content: AssistantMessageContent,
}

impl std::fmt::Debug for AssistantMessage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AssistantMessage")
            .field("role", &self.role)
            .field("content", &self.content)
            .finish()
    }
}

impl AssistantMessage {
    /// Create a user message.
    pub fn user(content: impl Into<AssistantMessageContent>) -> Self {
        Self {
            role: AssistantMessageRole::User,
            content: content.into(),
        }
    }
}

/// Translation options in an assistant request.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssistantTranslationParameters {
    /// Source language.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    /// Target language.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
}

impl std::fmt::Debug for AssistantTranslationParameters {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AssistantTranslationParameters")
            .field("from_configured", &self.from.is_some())
            .field("to_configured", &self.to.is_some())
            .finish()
    }
}

/// Typed extra parameters in an assistant request.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssistantExtraParameters {
    /// Translation configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translate: Option<AssistantTranslationParameters>,
}

impl std::fmt::Debug for AssistantExtraParameters {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AssistantExtraParameters")
            .field("translate_configured", &self.translate.is_some())
            .finish()
    }
}

/// Invoke an assistant.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct AssistantInvokeRequest {
    /// Assistant identifier.
    pub assistant_id: AssistantId,
    /// Existing conversation identifier, when continuing a conversation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
    /// Assistant model. This required field is always serialized.
    pub model: AssistantModel,
    /// User messages. The frozen schema requires at least one item.
    pub messages: Vec<AssistantMessage>,
    /// Streaming flag. The JSON-only `send_via` path requires this to remain
    /// `false` and serializes it explicitly because the server default is true.
    #[serde(default)]
    pub stream: bool,
    /// Caller-provided request identifier (`6..=64` characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// End-user identifier (`6..=128` characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    /// Whether sampling is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub do_sample: Option<bool>,
    /// Open attachment objects from the upstream schema.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<serde_json::Map<String, serde_json::Value>>>,
    /// Open metadata map (`additionalProperties: true`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Map<String, serde_json::Value>>,
    /// Typed assistant-specific parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_parameters: Option<AssistantExtraParameters>,
}

impl std::fmt::Debug for AssistantInvokeRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AssistantInvokeRequest")
            .field("assistant_id", &self.assistant_id)
            .field(
                "conversation_id_configured",
                &self.conversation_id.is_some(),
            )
            .field("model", &self.model)
            .field("message_count", &self.messages.len())
            .field("stream", &self.stream)
            .field("request_id_configured", &self.request_id.is_some())
            .field("user_id_configured", &self.user_id.is_some())
            .field("do_sample", &self.do_sample)
            .field(
                "attachment_count",
                &self.attachments.as_ref().map(std::vec::Vec::len),
            )
            .field(
                "metadata_field_count",
                &self.metadata.as_ref().map(serde_json::Map::len),
            )
            .field(
                "extra_parameters_configured",
                &self.extra_parameters.is_some(),
            )
            .finish()
    }
}

impl AssistantInvokeRequest {
    /// Create an invocation with all OpenAPI-required fields.
    pub fn new(assistant_id: AssistantId, messages: Vec<AssistantMessage>) -> Self {
        Self {
            assistant_id,
            conversation_id: None,
            model: AssistantModel::default(),
            messages,
            stream: false,
            request_id: None,
            user_id: None,
            do_sample: None,
            attachments: None,
            metadata: None,
            extra_parameters: None,
        }
    }

    /// Continue an existing conversation.
    pub fn with_conversation_id(mut self, conversation_id: impl Into<String>) -> Self {
        self.conversation_id = Some(conversation_id.into());
        self
    }

    /// Select the assistant model.
    pub fn with_model(mut self, model: AssistantModel) -> Self {
        self.model = model;
        self
    }

    /// Set the caller-provided request identifier.
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    /// Set the end-user identifier.
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Select the sampling behavior.
    pub fn with_sampling(mut self, enabled: bool) -> Self {
        self.do_sample = Some(enabled);
        self
    }

    /// Set attachment objects.
    pub fn with_attachments(
        mut self,
        attachments: Vec<serde_json::Map<String, serde_json::Value>>,
    ) -> Self {
        self.attachments = Some(attachments);
        self
    }

    /// Set open metadata fields.
    pub fn with_metadata(mut self, metadata: serde_json::Map<String, serde_json::Value>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Set typed assistant-specific parameters.
    pub fn with_extra_parameters(mut self, parameters: AssistantExtraParameters) -> Self {
        self.extra_parameters = Some(parameters);
        self
    }

    /// Validate OpenAPI length and minimum-item constraints.
    pub fn validate(&self) -> ZaiResult<()> {
        if self.stream {
            return Err(crate::client::validation::invalid(
                "assistant send_via supports only stream=false",
            ));
        }
        if self.messages.is_empty() {
            return Err(crate::client::validation::invalid(
                "assistant messages must contain at least one item",
            ));
        }
        if let Some(conversation_id) = self.conversation_id.as_deref() {
            crate::client::validation::require_non_blank(conversation_id, "conversation_id")?;
        }
        if self.request_id.as_ref().is_some_and(|value| {
            value.trim() != value || !(6..=64).contains(&value.chars().count())
        }) {
            return Err(crate::client::validation::invalid(
                "assistant request_id must contain between 6 and 64 characters",
            ));
        }
        if self.user_id.as_ref().is_some_and(|value| {
            value.trim() != value || !(6..=128).contains(&value.chars().count())
        }) {
            return Err(crate::client::validation::invalid(
                "assistant user_id must contain between 6 and 128 characters",
            ));
        }
        for message in &self.messages {
            message.content.validate()?;
        }
        if let Some(translation) = self
            .extra_parameters
            .as_ref()
            .and_then(|parameters| parameters.translate.as_ref())
        {
            for (value, name) in [
                (translation.from.as_deref(), "translation.from"),
                (translation.to.as_deref(), "translation.to"),
            ] {
                if let Some(value) = value {
                    crate::client::validation::require_non_blank(value, name)?;
                }
            }
        }
        Ok(())
    }

    /// Send the request through `client`.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<AssistantInvokeResponse> {
        self.validate()?;
        let route = crate::client::routes::ASSISTANTS_INVOKE;
        let response = client
            .operation(route)
            .send_json::<_, AssistantInvokeResponse>(self)
            .await?;
        response.validate()?;
        Ok(response)
    }
}

/// List assistants available to the current account.
#[derive(Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssistantListRequest {
    /// Assistant identifiers to query. An empty required array queries all
    /// assistants, matching the upstream default.
    pub assistant_id_list: Vec<String>,
}

impl std::fmt::Debug for AssistantListRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AssistantListRequest")
            .field("assistant_id_count", &self.assistant_id_list.len())
            .finish()
    }
}

impl AssistantListRequest {
    /// Create a request that lists all available assistants.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a request restricted to selected assistants.
    pub fn for_assistants(assistant_ids: Vec<String>) -> Self {
        Self {
            assistant_id_list: assistant_ids,
        }
    }

    /// Reject blank identifiers while preserving the documented empty-list
    /// meaning of "all assistants".
    pub fn validate(&self) -> ZaiResult<()> {
        self.assistant_id_list.iter().try_for_each(|assistant_id| {
            crate::client::validation::require_non_blank(assistant_id, "assistant_id")
        })
    }

    /// Send the request through `client`.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<AssistantListResponse> {
        self.validate()?;
        let route = crate::client::routes::ASSISTANTS_LIST;
        let response = client
            .operation(route)
            .send_json::<_, AssistantListResponse>(self)
            .await?;
        response.validate()?;
        Ok(response)
    }
}

/// List conversations for one assistant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssistantConversationListRequest {
    /// Assistant identifier.
    pub assistant_id: AssistantId,
    /// Page number (`>= 1`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    /// Page size (`1..=100`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<u32>,
}

impl AssistantConversationListRequest {
    /// Create a conversation-list request for one assistant.
    pub fn new(assistant_id: AssistantId) -> Self {
        Self {
            assistant_id,
            page: None,
            page_size: None,
        }
    }

    /// Set the page number.
    pub fn with_page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }

    /// Set the page size.
    pub fn with_page_size(mut self, page_size: u32) -> Self {
        self.page_size = Some(page_size);
        self
    }

    /// Validate OpenAPI pagination constraints.
    pub fn validate(&self) -> ZaiResult<()> {
        if self.page == Some(0) {
            return Err(crate::client::validation::invalid(
                "assistant page must be at least 1",
            ));
        }
        if self
            .page_size
            .is_some_and(|page_size| !(1..=100).contains(&page_size))
        {
            return Err(crate::client::validation::invalid(
                "assistant page_size must be between 1 and 100",
            ));
        }
        Ok(())
    }

    /// Send the request through `client`.
    pub async fn send_via(
        &self,
        client: &ZaiClient,
    ) -> ZaiResult<AssistantConversationListResponse> {
        self.validate()?;
        let route = crate::client::routes::ASSISTANTS_CONVERSATIONS;
        let response = client
            .operation(route)
            .send_json::<_, AssistantConversationListResponse>(self)
            .await?;
        response.validate()?;
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invoke_serializes_required_fields_and_the_content_union() {
        let request = AssistantInvokeRequest::new(
            AssistantId::ChatGlm,
            vec![AssistantMessage::user(vec![
                AssistantContentPart::text("describe"),
                AssistantContentPart::image_url("https://example.test/image.png"),
            ])],
        );
        assert_eq!(
            serde_json::to_value(request).unwrap(),
            serde_json::json!({
                "assistant_id": "65940acff94777010aa6b796",
                "model": "glm-4-assistant",
                "stream": false,
                "messages": [{
                    "role": "user",
                    "content": [
                        {"type": "text", "text": "describe"},
                        {"type": "image_url", "image_url": {"url": "https://example.test/image.png"}}
                    ]
                }]
            })
        );
    }

    #[test]
    fn assistant_list_keeps_its_required_empty_array() {
        assert_eq!(
            serde_json::to_value(AssistantListRequest::new()).unwrap(),
            serde_json::json!({"assistant_id_list": []})
        );
    }

    #[test]
    fn validation_enforces_frozen_shapes_and_numeric_constraints() {
        let mut streaming = AssistantInvokeRequest::new(
            AssistantId::ChatGlm,
            vec![AssistantMessage::user("hello")],
        );
        streaming.stream = true;
        assert!(streaming.validate().is_err());
        assert!(
            AssistantInvokeRequest::new(AssistantId::ChatGlm, Vec::new())
                .validate()
                .is_err()
        );
        assert!(
            AssistantInvokeRequest::new(
                AssistantId::ChatGlm,
                vec![AssistantMessage::user("hello")]
            )
            .with_request_id("short")
            .validate()
            .is_err()
        );
        assert!(
            AssistantInvokeRequest::new(AssistantId::ChatGlm, vec![AssistantMessage::user("   ")])
                .validate()
                .is_err()
        );
        assert!(
            AssistantInvokeRequest::new(
                AssistantId::ChatGlm,
                vec![AssistantMessage::user(vec![AssistantContentPart {
                    type_: Some(AssistantContentType::Text),
                    text: None,
                    image_url: None,
                }])]
            )
            .validate()
            .is_err()
        );
        assert!(
            AssistantListRequest::for_assistants(vec![" ".to_owned()])
                .validate()
                .is_err()
        );
        assert!(
            AssistantConversationListRequest::new(AssistantId::ChatGlm)
                .with_page(0)
                .validate()
                .is_err()
        );
        assert!(
            AssistantConversationListRequest::new(AssistantId::ChatGlm)
                .with_page_size(101)
                .validate()
                .is_err()
        );
    }

    #[test]
    fn debug_output_redacts_messages_urls_and_open_maps() {
        let mut attachment = serde_json::Map::new();
        attachment.insert(
            "private-key".into(),
            serde_json::json!("private-attachment"),
        );
        let mut metadata = serde_json::Map::new();
        metadata.insert(
            "private-meta-key".into(),
            serde_json::json!("private-metadata"),
        );
        let request = AssistantInvokeRequest::new(
            AssistantId::ChatGlm,
            vec![AssistantMessage::user(vec![
                AssistantContentPart::text("private-text"),
                AssistantContentPart::image_url("https://private.example/image.png"),
            ])],
        )
        .with_conversation_id("private-conversation")
        .with_request_id("private-request")
        .with_user_id("private-user")
        .with_attachments(vec![attachment])
        .with_metadata(metadata)
        .with_extra_parameters(AssistantExtraParameters {
            translate: Some(AssistantTranslationParameters {
                from: Some("private-source".into()),
                to: Some("private-target".into()),
            }),
        });
        let debug = format!("{request:?}");
        for secret in [
            "private-text",
            "private.example",
            "private-conversation",
            "private-request",
            "private-user",
            "private-key",
            "private-attachment",
            "private-meta-key",
            "private-metadata",
            "private-source",
            "private-target",
        ] {
            assert!(!debug.contains(secret), "Debug leaked {secret}");
        }
    }
}
