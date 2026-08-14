//! Shared chat-completion request body.
//!
//! [`ChatBody`](crate::model::chat_base_request::ChatBody) is used by chat-completion
//! endpoints. The generic type parameters enforce compile-time compatibility
//! between model and message types.

use serde::Serialize;
use validator::*;

use super::{tools::*, traits::*};

/// Validated request body for a chat-completion operation.
///
/// This type stores the union of frozen chat request fields. Its fields
/// are private, and capability-gated builders expose only the subset accepted
/// by the selected text, vision, or audio model family.
///
/// # Examples
///
/// ```
/// use zai_rs::model::{
///     chat_base_request::ChatBody,
///     chat_message_types::TextMessage,
///     chat_models::GLM5_2,
/// };
///
/// let chat_body = ChatBody::new(GLM5_2 {}, TextMessage::user("Hello"))
///     .add_message(TextMessage::assistant("How can I help?"))
///     .with_temperature(0.7)
///     .with_max_tokens(1_000);
/// ```
#[derive(Clone, Validate, Serialize)]
#[validate(schema(function = "validate_chat_body"))]
pub struct ChatBody<N, M>
where
    N: ChatRequestModel,
    M: Serialize,
    (N, M): Bounded,
{
    /// The model to use for the chat completion.
    model: N,

    /// A list of messages comprising the conversation so far.
    #[validate(length(min = 1))]
    messages: Vec<M>,

    /// A unique identifier for the request. Optional field that will be omitted
    /// from serialization if not provided.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 6, max = 64))]
    request_id: Option<String>,

    /// Thinking-mode configuration for models that support extended reasoning.
    ///
    /// GLM-5.3 always thinks: setting `type = "disabled"` fails validation
    /// with `thinking_cannot_be_disabled`. Migrate legacy requests to
    /// `enabled` plus `reasoning_effort = "low"` for the lightest behaviour.
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking: Option<ThinkingType>,

    /// Controls the depth of reasoning when thinking mode is enabled. Only
    /// available for GLM-5.2 and above (models that implement
    /// `ReasoningEffortEnable`). See [`ReasoningEffort`] for the available
    /// levels; each model accepts its own subset, frozen in
    /// [`ChatRequestModel::REASONING_EFFORTS`](crate::model::traits::ChatRequestModel::REASONING_EFFORTS)
    /// — GLM-5.3 accepts only `low`, `high`, and `max`.
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<ReasoningEffort>,

    /// Whether to use sampling during generation. When `true`, the model will
    /// use probabilistic sampling; when `false`, it will use deterministic
    /// generation.
    #[serde(skip_serializing_if = "Option::is_none")]
    do_sample: Option<bool>,

    /// Whether to stream back partial message deltas as they are generated.
    /// When `true`, responses will be sent as server-sent events.
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,

    /// Whether to enable streaming of tool calls (streaming function call
    /// parameters). Supported by GLM-5.3, GLM-5.2, GLM-5.1, GLM-5, GLM-5-Turbo,
    /// GLM-4.7, and GLM-4.6 models. Defaults to false when omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_stream: Option<bool>,

    /// Controls randomness in the output. Higher values (closer to 1.0) make
    /// the output more random, while lower values (closer to 0.0) make it
    /// more deterministic. Must be between 0.0 and 1.0.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 0.0, max = 1.0))]
    temperature: Option<f64>,

    /// Controls diversity via nucleus sampling. Only tokens with cumulative
    /// probability up to `top_p` are considered. Must be between 0.0 and
    /// 1.0, with a minimum non-zero value of 0.01.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 0.01, max = 1.0))]
    top_p: Option<f64>,

    /// The maximum number of tokens to generate in the completion.
    /// Must be between 1 and 131,072. Individual models may impose a smaller
    /// service-side maximum.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 1, max = 131072))]
    max_tokens: Option<u32>,

    /// A list of tools the model may call. Currently supports function calling,
    /// web search, and retrieval tools.
    /// Note: server expects an array; we model this as a vector of tool items.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(max = 128))]
    tools: Option<Vec<Tools>>,

    /// Controls whether the model calls a tool, and which one. Only meaningful
    /// when [`tools`](Self::tools) is set. See [`ToolChoice`] for the wire
    /// form (`"auto"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<ToolChoice>,

    /// A unique identifier representing your end-user, which can help monitor
    /// and detect abuse. Must be between 6 and 128 characters long.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 6, max = 128))]
    user_id: Option<String>,

    /// Up to four non-empty sequences where the API will stop generation.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 1, max = 4))]
    stop: Option<Vec<String>>,

    /// An object specifying the format that the model must output.
    /// Can be either text or JSON object format.
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,

    /// Whether generated audio-model media carries the provider watermark.
    #[serde(skip_serializing_if = "Option::is_none")]
    watermark_enabled: Option<bool>,
}

impl<N, M> std::fmt::Debug for ChatBody<N, M>
where
    N: ChatRequestModel + std::fmt::Debug,
    M: Serialize,
    (N, M): Bounded,
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ChatBody")
            .field("model", &self.model)
            .field("messages_count", &self.messages.len())
            .field("request_id_set", &self.request_id.is_some())
            .field("thinking", &self.thinking)
            .field("reasoning_effort", &self.reasoning_effort)
            .field("do_sample", &self.do_sample)
            .field("stream", &self.stream)
            .field("tool_stream", &self.tool_stream)
            .field("temperature", &self.temperature)
            .field("top_p", &self.top_p)
            .field("max_tokens", &self.max_tokens)
            .field("tools_count", &self.tools.as_ref().map(std::vec::Vec::len))
            .field("tool_choice", &self.tool_choice)
            .field("user_id_set", &self.user_id.is_some())
            .field("stop_count", &self.stop.as_ref().map(std::vec::Vec::len))
            .field("response_format", &self.response_format)
            .field("watermark_enabled", &self.watermark_enabled)
            .finish()
    }
}

fn validate_chat_body<N, M>(body: &ChatBody<N, M>) -> Result<(), ValidationError>
where
    N: ChatRequestModel,
    M: Serialize,
    (N, M): Bounded,
{
    if body
        .temperature
        .is_some_and(|temperature| !temperature.is_finite())
    {
        return Err(ValidationError::new("temperature_must_be_finite"));
    }
    if body.top_p.is_some_and(|top_p| !top_p.is_finite()) {
        return Err(ValidationError::new("top_p_must_be_finite"));
    }
    if !N::THINKING_DISABLE_SUPPORTED
        && body
            .thinking
            .as_ref()
            .is_some_and(|thinking| thinking.mode == ThinkingMode::Disabled)
    {
        return Err(ValidationError::new("thinking_cannot_be_disabled"));
    }
    if body
        .reasoning_effort
        .is_some_and(|effort| !N::REASONING_EFFORTS.contains(&effort))
    {
        return Err(ValidationError::new("reasoning_effort_not_supported"));
    }
    if body
        .max_tokens
        .is_some_and(|max_tokens| max_tokens > N::MAX_TOKENS)
    {
        return Err(ValidationError::new("max_tokens_exceeds_model_limit"));
    }
    if body
        .stop
        .as_ref()
        .is_some_and(|values| values.iter().any(|value| value.trim().is_empty()))
    {
        return Err(ValidationError::new("stop_must_not_be_blank"));
    }
    if body
        .tools
        .as_ref()
        .is_some_and(|tools| tools.iter().any(|tool| tool.validate().is_err()))
    {
        return Err(ValidationError::new("invalid_tool"));
    }
    if body.tool_choice.is_some() && body.tools.as_ref().is_none_or(Vec::is_empty) {
        return Err(ValidationError::new("tool_choice_requires_tools"));
    }
    Ok(())
}

impl<N, M> ChatBody<N, M>
where
    N: ChatRequestModel,
    M: Serialize,
    (N, M): Bounded,
{
    /// Create a new chat request body from a model and the first message batch.
    pub fn new(model: N, messages: M) -> Self {
        Self {
            model,
            messages: vec![messages],
            request_id: None,
            thinking: None,
            reasoning_effort: None,
            do_sample: None,
            stream: None,
            tool_stream: None,
            temperature: None,
            top_p: None,
            max_tokens: None,
            tools: None,
            tool_choice: None,
            user_id: None,
            stop: None,
            response_format: None,
            watermark_enabled: None,
        }
    }

    /// Add one message to the conversation.
    pub fn add_message(mut self, message: M) -> Self {
        self.messages.push(message);
        self
    }
    /// Add multiple messages to the conversation at once.
    pub fn extend_messages(mut self, messages: impl IntoIterator<Item = M>) -> Self {
        self.messages.extend(messages);
        self
    }
    /// Set the client-side request id (used for tracing/dedup).
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }
    /// Enable/disable sampling (`do_sample`).
    pub fn with_do_sample(mut self, do_sample: bool) -> Self {
        self.do_sample = Some(do_sample);
        self
    }
    /// Set the sampling temperature.
    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }
    /// Set the nucleus-sampling probability (`top_p`).
    pub fn with_top_p(mut self, top_p: f64) -> Self {
        self.top_p = Some(top_p);
        self
    }
    /// Set the maximum number of tokens to generate.
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }
    /// Set the end-user id (used for abuse monitoring).
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }
    /// Add a stop sequence that halts generation when encountered.
    pub fn with_stop(mut self, stop: impl Into<String>) -> Self {
        self.stop.get_or_insert_with(Vec::new).push(stop.into());
        self
    }

    /// Borrow the selected model.
    pub const fn model(&self) -> &N {
        &self.model
    }

    /// Borrow the complete conversation in wire order.
    pub fn messages(&self) -> &[M] {
        &self.messages
    }

    /// Client-provided request identifier.
    pub fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }

    /// Thinking configuration, when supported and configured.
    pub const fn thinking(&self) -> Option<&ThinkingType> {
        self.thinking.as_ref()
    }

    /// Reasoning depth, when supported and configured.
    pub const fn reasoning_effort(&self) -> Option<ReasoningEffort> {
        self.reasoning_effort
    }

    /// Explicit sampling toggle.
    pub const fn do_sample(&self) -> Option<bool> {
        self.do_sample
    }

    /// Streaming flag managed by the [`ChatCompletion`](super::chat::ChatCompletion)
    /// type state.
    pub const fn stream(&self) -> Option<bool> {
        self.stream
    }

    /// Incremental tool-call flag managed by the streaming builder.
    pub const fn tool_stream(&self) -> Option<bool> {
        self.tool_stream
    }

    /// Configured sampling temperature.
    pub const fn temperature(&self) -> Option<f64> {
        self.temperature
    }

    /// Configured nucleus-sampling probability.
    pub const fn top_p(&self) -> Option<f64> {
        self.top_p
    }

    /// Configured output-token limit.
    pub const fn max_tokens(&self) -> Option<u32> {
        self.max_tokens
    }

    /// Attached tools, if this model family supports them.
    pub fn tools(&self) -> Option<&[Tools]> {
        self.tools.as_deref()
    }

    /// Configured automatic tool-selection policy.
    pub const fn tool_choice(&self) -> Option<ToolChoice> {
        self.tool_choice
    }

    /// End-user identifier used for abuse monitoring.
    pub fn user_id(&self) -> Option<&str> {
        self.user_id.as_deref()
    }

    /// Borrow configured stop sequences.
    pub fn stop(&self) -> Option<&[String]> {
        self.stop.as_deref()
    }

    /// Text response format, when supported and configured.
    pub const fn response_format(&self) -> Option<ResponseFormat> {
        self.response_format
    }

    /// Audio-model watermark setting, when configured.
    pub const fn watermark_enabled(&self) -> Option<bool> {
        self.watermark_enabled
    }

    pub(crate) fn set_stream(&mut self, stream: Option<bool>) {
        self.stream = stream;
    }

    pub(crate) fn clear_tool_stream(&mut self) {
        self.tool_stream = None;
    }
}

impl<N, M> ChatBody<N, M>
where
    N: ChatToolSupport,
    M: Serialize,
    (N, M): Bounded,
{
    /// Add one schema-compatible tool.
    pub fn add_tool(mut self, tool: N::Tool) -> Self {
        self.tools.get_or_insert_default().push(tool.into());
        self
    }

    /// Add multiple schema-compatible tools.
    pub fn add_tools(mut self, tools: impl IntoIterator<Item = N::Tool>) -> Self {
        self.tools
            .get_or_insert_default()
            .extend(tools.into_iter().map(Into::into));
        self
    }

    /// Let the model choose automatically among the attached tools.
    pub fn with_tool_choice(mut self, tool_choice: ToolChoice) -> Self {
        self.tool_choice = Some(tool_choice);
        self
    }

    /// Remove attached tools and their selection policy.
    pub fn clear_tools(mut self) -> Self {
        self.tools = None;
        self.tool_choice = None;
        self
    }
}

impl<N, M> ChatBody<N, M>
where
    N: ResponseFormatEnable,
    M: Serialize,
    (N, M): Bounded,
{
    /// Set the response format for a text chat model.
    pub fn with_response_format(mut self, format: ResponseFormat) -> Self {
        self.response_format = Some(format);
        self
    }
}

impl<N, M> ChatBody<N, M>
where
    N: WatermarkEnable,
    M: Serialize,
    (N, M): Bounded,
{
    /// Enable or disable provider watermarking for audio-model output.
    pub fn with_watermark_enabled(mut self, enabled: bool) -> Self {
        self.watermark_enabled = Some(enabled);
        self
    }
}

impl<N, M> ChatBody<N, M>
where
    N: ChatRequestModel + ThinkEnable,
    M: Serialize,
    (N, M): Bounded,
{
    /// Configure extended reasoning for a model that implements
    /// [`ThinkEnable`].
    ///
    /// # Examples
    ///
    /// ```
    /// use zai_rs::model::{
    ///     chat_base_request::ChatBody,
    ///     chat_message_types::TextMessage,
    ///     chat_models::GLM5_2,
    ///     tools::ThinkingType,
    /// };
    ///
    /// let chat_body = ChatBody::new(GLM5_2 {}, TextMessage::user("Solve this"))
    ///     .with_thinking(ThinkingType::enabled());
    /// ```
    pub fn with_thinking(mut self, thinking: ThinkingType) -> Self {
        self.thinking = Some(thinking);
        self
    }
}

impl<N, M> ChatBody<N, M>
where
    N: ChatRequestModel + ReasoningEffortEnable,
    M: Serialize,
    (N, M): Bounded,
{
    /// Set the `reasoning_effort` level that controls how much reasoning the
    /// model performs when thinking mode is enabled.
    ///
    /// Available only for models implementing [`ReasoningEffortEnable`].
    /// Typically combined with
    /// [`with_thinking`](Self::with_thinking) to enable thinking first.
    /// GLM-5.3 accepts only [`ReasoningEffort::Low`], [`ReasoningEffort::High`],
    /// and [`ReasoningEffort::Max`]; other levels fail validation with
    /// `reasoning_effort_not_supported`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zai_rs::model::{
    ///     GLM5_2, TextMessage,
    ///     chat_base_request::ChatBody,
    ///     tools::{ReasoningEffort, ThinkingType},
    /// };
    ///
    /// let chat_body = ChatBody::new(GLM5_2 {}, TextMessage::user("Solve this"))
    ///     .with_thinking(ThinkingType::enabled())
    ///     .with_reasoning_effort(ReasoningEffort::Max);
    /// ```
    pub fn with_reasoning_effort(mut self, effort: ReasoningEffort) -> Self {
        self.reasoning_effort = Some(effort);
        self
    }
}

impl<N, M> ChatBody<N, M>
where
    N: ToolStreamEnable,
    M: Serialize,
    (N, M): Bounded,
{
    /// Enable streaming tool calls and the containing chat stream.
    pub(crate) fn with_tool_stream(mut self, tool_stream: bool) -> Self {
        self.tool_stream = Some(tool_stream);
        if tool_stream {
            self.stream = Some(true);
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        chat_message_types::{TextMessage, VisionMessage, VoiceMessage},
        chat_models::{GLM4_5_air, GLM4_6, GLM4_6v, GLM4_voice, GLM5_2, GLM5_3},
    };
    use validator::Validate;

    #[test]
    fn test_with_tool_stream_sets_both_fields() {
        let body: ChatBody<GLM4_6, TextMessage> =
            ChatBody::new(GLM4_6 {}, TextMessage::user("test"));
        let body = body.with_tool_stream(true);
        assert_eq!(body.tool_stream(), Some(true));
        assert_eq!(body.stream(), Some(true));
    }

    #[test]
    fn test_with_tool_stream_false_does_not_force_stream() {
        let body: ChatBody<GLM4_6, TextMessage> =
            ChatBody::new(GLM4_6 {}, TextMessage::user("test"));
        let body = body.with_tool_stream(false);
        assert_eq!(body.tool_stream(), Some(false));
        // stream should not be forced to true when tool_stream is false
        assert_ne!(body.stream(), Some(true));
    }

    #[test]
    fn test_add_tool_stores_function_tool() {
        let body: ChatBody<GLM4_6, TextMessage> =
            ChatBody::new(GLM4_6 {}, TextMessage::user("test"));
        let tool = crate::model::tools::Function::new(
            "test_fn",
            "A test function",
            serde_json::json!({"type": "object"}),
        );
        let body = body.add_tool(crate::model::tools::Tools::Function { function: tool });
        assert_eq!(body.tools().map(|tools| tools.len()), Some(1));
    }

    #[test]
    fn test_extend_messages() {
        let body: ChatBody<GLM4_6, TextMessage> =
            ChatBody::new(GLM4_6 {}, TextMessage::user("first"));
        let body = body.extend_messages(vec![
            TextMessage::assistant("second"),
            TextMessage::user("third"),
        ]);
        assert_eq!(body.messages().len(), 3);
        match &body.messages()[0] {
            TextMessage::User { content } => assert_eq!(content, "first"),
            _ => panic!("Expected User message"),
        }
    }

    #[test]
    fn test_add_message() {
        let body: ChatBody<GLM4_6, TextMessage> =
            ChatBody::new(GLM4_6 {}, TextMessage::user("first"));
        let body = body.add_message(TextMessage::assistant("second"));
        assert_eq!(body.messages().len(), 2);
    }

    #[test]
    fn test_glm52_reasoning_effort_builder() {
        // reasoning_effort is only available on GLM-5.2+ (ReasoningEffortEnable)
        let body: ChatBody<GLM5_2, TextMessage> = ChatBody::new(GLM5_2 {}, TextMessage::user("hi"))
            .with_thinking(ThinkingType::enabled())
            .with_reasoning_effort(ReasoningEffort::Max);
        assert_eq!(body.reasoning_effort(), Some(ReasoningEffort::Max));
        assert!(body.thinking().is_some());
    }

    #[test]
    fn test_glm52_serializes_model_name() {
        let body: ChatBody<GLM5_2, TextMessage> = ChatBody::new(GLM5_2 {}, TextMessage::user("hi"));
        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(json["model"], "glm-5.2");
        // reasoning_effort must be omitted when not set
        assert!(json.get("reasoning_effort").is_none());
    }

    #[test]
    fn test_sampling_parameters_serialize_without_f32_expansion() {
        let body: ChatBody<GLM4_5_air, TextMessage> =
            ChatBody::new(GLM4_5_air {}, TextMessage::user("hi"))
                .with_temperature(0.7)
                .with_top_p(0.9);

        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(json["temperature"], serde_json::json!(0.7));
        assert_eq!(json["top_p"], serde_json::json!(0.9));

        let serialized = serde_json::to_string(&body).unwrap();
        assert!(serialized.contains("\"temperature\":0.7"));
        assert!(serialized.contains("\"top_p\":0.9"));
        assert!(!serialized.contains("0.699999"));
        assert!(!serialized.contains("0.899999"));
    }

    #[test]
    fn test_reasoning_effort_serializes_level() {
        let body: ChatBody<GLM5_2, TextMessage> = ChatBody::new(GLM5_2 {}, TextMessage::user("hi"))
            .with_reasoning_effort(ReasoningEffort::Xhigh);
        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(json["reasoning_effort"], "xhigh");
    }

    #[test]
    fn test_tool_choice_is_omitted_by_default() {
        let body: ChatBody<GLM4_6, TextMessage> = ChatBody::new(GLM4_6 {}, TextMessage::user("hi"));
        assert!(body.tool_choice().is_none());
        let json = serde_json::to_value(&body).unwrap();
        assert!(json.get("tool_choice").is_none());
    }

    #[test]
    fn test_with_tool_choice_serializes_only_auto() {
        let body: ChatBody<GLM4_6, TextMessage> = ChatBody::new(GLM4_6 {}, TextMessage::user("hi"))
            .add_tool(Tools::Function {
                function: Function::new("lookup", "Lookup", serde_json::json!({"type": "object"})),
            })
            .with_tool_choice(crate::model::tools::ToolChoice::auto());
        assert!(body.validate().is_ok());
        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(json["tool_choice"], serde_json::json!("auto"));
    }

    #[test]
    fn test_with_response_format_serializes() {
        use crate::model::tools::ResponseFormat;
        let body: ChatBody<GLM4_6, TextMessage> = ChatBody::new(GLM4_6 {}, TextMessage::user("hi"))
            .with_response_format(ResponseFormat::JsonObject);
        assert_eq!(body.response_format(), Some(ResponseFormat::JsonObject));
        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(json["response_format"]["type"], "json_object");
    }

    #[test]
    fn validation_matches_current_chat_limits() {
        let body: ChatBody<GLM4_6, TextMessage> = ChatBody::new(GLM4_6 {}, TextMessage::user("hi"))
            .with_request_id("123456")
            .with_top_p(0.01)
            .with_max_tokens(131_072)
            .with_stop("one")
            .with_stop("two")
            .with_stop("three")
            .with_stop("four");
        assert!(body.validate().is_ok());

        assert!(body.clone().with_stop("five").validate().is_err());
        assert!(body.clone().with_stop(" ").validate().is_err());
        assert!(body.clone().with_top_p(0.0).validate().is_err());
        assert!(body.clone().with_temperature(f64::NAN).validate().is_err());
        assert!(body.with_request_id("short").validate().is_err());
    }

    #[test]
    fn glm53_rejects_disabling_thinking() {
        let body = ChatBody::new(GLM5_3 {}, TextMessage::user("hi"));
        assert!(body.clone().validate().is_ok());
        assert!(
            body.clone()
                .with_thinking(ThinkingType::enabled())
                .validate()
                .is_ok()
        );

        // GLM-5.3 dropped thinking.type = "disabled"; validation must fail
        // before the request leaves the client.
        let disabled = body.with_thinking(ThinkingType::disabled());
        let errors = disabled
            .validate()
            .expect_err("disabled thinking must fail validation");
        assert!(format!("{errors:?}").contains("thinking_cannot_be_disabled"));

        // Disabling thinking stays valid on models that support it.
        let glm46 = ChatBody::new(GLM4_6 {}, TextMessage::user("hi"))
            .with_thinking(ThinkingType::disabled());
        assert!(glm46.validate().is_ok());
    }

    #[test]
    fn glm53_accepts_only_low_high_max_reasoning_efforts() {
        for effort in [
            ReasoningEffort::Low,
            ReasoningEffort::High,
            ReasoningEffort::Max,
        ] {
            let body = ChatBody::new(GLM5_3 {}, TextMessage::user("hi"))
                .with_thinking(ThinkingType::enabled())
                .with_reasoning_effort(effort);
            assert!(body.validate().is_ok(), "{effort:?} must be accepted");
        }

        for effort in [
            ReasoningEffort::Xhigh,
            ReasoningEffort::Medium,
            ReasoningEffort::Minimal,
            ReasoningEffort::None,
        ] {
            let body =
                ChatBody::new(GLM5_3 {}, TextMessage::user("hi")).with_reasoning_effort(effort);
            let errors = body
                .validate()
                .expect_err("unsupported effort must fail validation");
            assert!(
                format!("{errors:?}").contains("reasoning_effort_not_supported"),
                "{effort:?} must be rejected with reasoning_effort_not_supported"
            );
        }
    }

    #[test]
    fn glm52_keeps_every_reasoning_effort_level() {
        for effort in [
            ReasoningEffort::Max,
            ReasoningEffort::Xhigh,
            ReasoningEffort::High,
            ReasoningEffort::Medium,
            ReasoningEffort::Low,
            ReasoningEffort::Minimal,
            ReasoningEffort::None,
        ] {
            let body = ChatBody::new(GLM5_2 {}, TextMessage::user("hi"))
                .with_thinking(ThinkingType::enabled())
                .with_reasoning_effort(effort);
            assert!(body.validate().is_ok(), "{effort:?} must be accepted");
        }
    }

    #[test]
    fn glm53_migration_shape_serializes_the_recommended_request() {
        // Official migration guidance: replace thinking.type = "disabled"
        // with enabled + reasoning_effort = low; the recommended shape for
        // coding tasks is enabled + reasoning_effort = max.
        let light = ChatBody::new(GLM5_3 {}, TextMessage::user("hi"))
            .with_thinking(ThinkingType::enabled())
            .with_reasoning_effort(ReasoningEffort::Low);
        assert!(light.validate().is_ok());

        let deep = ChatBody::new(GLM5_3 {}, TextMessage::user("Refactor this module"))
            .with_thinking(ThinkingType::enabled())
            .with_reasoning_effort(ReasoningEffort::Max);
        assert!(deep.validate().is_ok());

        let json = serde_json::to_value(&deep).unwrap();
        assert_eq!(json["model"], "glm-5.3");
        assert_eq!(json["thinking"]["type"], "enabled");
        assert_eq!(json["reasoning_effort"], "max");
    }

    #[test]
    fn vision_accepts_only_the_function_tool_wire_shape() {
        let body = ChatBody::new(GLM4_6v {}, VisionMessage::new_user())
            .add_tool(Function::new(
                "inspect",
                "Inspect the image",
                serde_json::json!({"type": "object"}),
            ))
            .with_tool_choice(ToolChoice::auto());
        assert!(body.validate().is_ok());

        let json = serde_json::to_value(body).unwrap();
        assert_eq!(json["tools"][0]["type"], "function");
        assert_eq!(json["tool_choice"], "auto");
        assert!(json.get("response_format").is_none());
        assert!(json.get("watermark_enabled").is_none());
    }

    #[test]
    fn audio_exposes_watermark_and_enforces_its_token_limit() {
        let body = ChatBody::new(GLM4_voice {}, VoiceMessage::new_user())
            .with_max_tokens(4_096)
            .with_watermark_enabled(false);
        assert!(body.validate().is_ok());
        assert!(body.clone().with_max_tokens(4_097).validate().is_err());

        let json = serde_json::to_value(body).unwrap();
        assert_eq!(json["watermark_enabled"], false);
        for field in [
            "thinking",
            "reasoning_effort",
            "tools",
            "tool_choice",
            "tool_stream",
            "response_format",
        ] {
            assert!(json.get(field).is_none(), "unexpected audio field: {field}");
        }
    }

    #[test]
    fn debug_redacts_conversation_and_caller_identifiers() {
        let body = ChatBody::new(GLM5_2 {}, TextMessage::user("prompt-secret-7cc8fd03"))
            .with_request_id("request-secret-7cc8fd03")
            .with_user_id("user-secret-7cc8fd03")
            .with_stop("stop-secret-7cc8fd03")
            .with_temperature(0.4);

        let debug = format!("{body:?}");
        for secret in [
            "prompt-secret-7cc8fd03",
            "request-secret-7cc8fd03",
            "user-secret-7cc8fd03",
            "stop-secret-7cc8fd03",
        ] {
            assert!(!debug.contains(secret));
        }
        assert!(debug.contains("GLM5_2"));
        assert!(debug.contains("messages_count: 1"));
        assert!(debug.contains("request_id_set: true"));
        assert!(debug.contains("user_id_set: true"));
        assert!(debug.contains("stop_count: Some(1)"));
        assert!(debug.contains("temperature: Some(0.4)"));
    }

    #[test]
    fn validation_rejects_invalid_nested_tools() {
        let body: ChatBody<GLM4_6, TextMessage> = ChatBody::new(GLM4_6 {}, TextMessage::user("hi"))
            .add_tool(Tools::Function {
                function: Function::new("", "invalid", serde_json::json!({"type": "object"})),
            });
        assert!(body.validate().is_err());
    }

    #[test]
    fn validation_requires_tools_when_auto_choice_is_explicit() {
        let body: ChatBody<GLM4_6, TextMessage> =
            ChatBody::new(GLM4_6 {}, TextMessage::user("hi")).with_tool_choice(ToolChoice::auto());
        assert!(body.validate().is_err());

        let function = Function::new("found", "valid", serde_json::json!({"type": "object"}));
        let body = body
            .add_tool(Tools::Function { function })
            .with_tool_choice(ToolChoice::auto());
        assert!(body.validate().is_ok());

        let json = serde_json::to_value(body.clear_tools()).unwrap();
        assert!(json.get("tools").is_none());
        assert!(json.get("tool_choice").is_none());
    }
}
