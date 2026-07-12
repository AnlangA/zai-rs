//! # Tool Definitions & Configurations
//!
//! Defines the tool types that can be attached to chat requests, including
//! function calling, web search integration, retrieval tools, and the
//! [`ThinkingType`] configuration.
//!
//! # Key Types
//!
//! - [`ThinkingType`] — Controls reasoning mode for thinking-capable models
//! - [`ReasoningEffort`] — Controls reasoning depth for GLM-5.2+ models
//! - `Function` — Defines a callable function with JSON-schema parameters
//! - `WebSearch` — Enables live web search within chat
//! - [`Retrieval`] — Enables knowledge-base retrieval
//! - [`ToolChoice`] — Enables the frozen automatic tool-selection policy

use std::collections::BTreeMap;

use serde::Serialize;
use validator::Validate;

use super::model_validate::validate_json_schema_value;
use crate::tool::web_search::{ContentSize, SearchEngine, SearchRecencyFilter};

/// Controls extended reasoning and whether reasoning context is cleared between
/// turns.
///
/// # Examples
///
/// ```
/// use zai_rs::model::{
///     chat::ChatCompletion,
///     chat_message_types::TextMessage,
///     chat_models::GLM5_2,
///     tools::ThinkingType,
/// };
///
/// let request = ChatCompletion::new(GLM5_2 {}, TextMessage::user("Solve this"))
///     .with_thinking(ThinkingType::enabled());
///
/// let request = ChatCompletion::new(GLM5_2 {}, TextMessage::user("Continue"))
///     .with_thinking(ThinkingType::enabled().with_clear_thinking(false));
/// ```
///
/// # Model compatibility
///
/// Thinking capabilities are available only on models that implement the
/// `ThinkEnable` trait, such as GLM-5.2, GLM-5.1, GLM-5, GLM-4.7, and GLM-4.5
/// series models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ThinkingType {
    /// Whether thinking is enabled or disabled.
    #[serde(rename = "type")]
    pub mode: ThinkingMode,

    /// Whether to clear historical `reasoning_content`.
    ///
    /// - `true` (default for standard API): Clears reasoning content each turn.
    /// - `false` (recommended for Coding / Agent): Preserves reasoning content
    ///   across turns, enabling better context for multi-step tool calls.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clear_thinking: Option<bool>,
}

/// Thinking mode variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ThinkingMode {
    /// Extended reasoning is enabled; exposure of reasoning text depends on the
    /// selected model and endpoint.
    Enabled,
    /// Extended reasoning is disabled.
    Disabled,
}

impl ThinkingType {
    /// Create a new thinking configuration with enabled mode.
    pub fn enabled() -> Self {
        Self {
            mode: ThinkingMode::Enabled,
            clear_thinking: None,
        }
    }

    /// Create a new thinking configuration with disabled mode.
    pub fn disabled() -> Self {
        Self {
            mode: ThinkingMode::Disabled,
            clear_thinking: None,
        }
    }

    /// Set whether to clear historical reasoning content.
    ///
    /// Use `false` for Coding / Agent scenarios where reasoning content should
    /// be preserved across turns.
    pub fn with_clear_thinking(mut self, clear: bool) -> Self {
        self.clear_thinking = Some(clear);
        self
    }
}

/// Reasoning depth level for the `reasoning_effort` parameter (GLM-5.2+).
///
/// Controls how much reasoning the model invests when thinking mode is
/// enabled. Higher levels yield deeper reasoning at the cost of latency and
/// token usage; lower levels are faster and cheaper. Available only on
/// GLM-5.2 and above (models implementing
/// [`ReasoningEffortEnable`](super::traits::ReasoningEffortEnable)).
///
/// Levels, from highest to lowest reasoning depth:
///
/// | Variant | Description |
/// |---------|-------------|
/// | [`Max`](Self::Max) | Maximum reasoning depth; recommended for coding / architecture-level tasks |
/// | [`Xhigh`](Self::Xhigh) | Extra-high reasoning |
/// | [`High`](Self::High) | High reasoning (default mapping in many clients) |
/// | [`Medium`](Self::Medium) | Balanced reasoning |
/// | [`Low`](Self::Low) | Light reasoning |
/// | [`Minimal`](Self::Minimal) | Minimal reasoning |
/// | [`None`](Self::None) | No extra reasoning beyond base behaviour |
///
/// ## Usage
///
/// ```
/// use zai_rs::model::{
///     chat::ChatCompletion,
///     chat_message_types::TextMessage,
///     chat_models::GLM5_2,
///     tools::{ReasoningEffort, ThinkingType},
/// };
///
/// let request = ChatCompletion::new(GLM5_2 {}, TextMessage::user("Design an API"))
///     .with_thinking(ThinkingType::enabled())
///     .with_reasoning_effort(ReasoningEffort::Max);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ReasoningEffort {
    /// Maximum reasoning depth. Recommended for coding and architecture-level
    /// tasks where correctness matters most.
    #[serde(rename = "max")]
    Max,
    /// Extra-high reasoning depth.
    #[serde(rename = "xhigh")]
    Xhigh,
    /// High reasoning depth.
    #[serde(rename = "high")]
    High,
    /// Balanced reasoning depth.
    #[serde(rename = "medium")]
    Medium,
    /// Light reasoning depth.
    #[serde(rename = "low")]
    Low,
    /// Minimal reasoning depth.
    #[serde(rename = "minimal")]
    Minimal,
    /// No extra reasoning beyond base behaviour.
    #[serde(rename = "none")]
    None,
}

/// External capability attached to a chat request.
///
/// Variants cover caller-defined functions, knowledge retrieval, web search,
/// and Model Context Protocol servers.
///
/// # Examples
///
/// ```
/// use zai_rs::model::tools::{Function, Tools, WebSearch};
/// use zai_rs::tool::web_search::SearchEngine;
///
/// let function_tool = Tools::Function {
///     function: Function::new(
///         "get_weather",
///         "Get weather data",
///         serde_json::json!({"type": "object"}),
///     ),
/// };
///
/// let search_tool = Tools::WebSearch {
///     web_search: WebSearch::new(SearchEngine::SearchPro)
///         .with_enable(true)
///         .with_count(10)
/// };
/// ```
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum Tools {
    /// Custom function calling tool with parameters.
    ///
    /// Allows the AI to invoke user-defined functions with structured
    /// arguments. Functions must be pre-defined with JSON schemas for
    /// parameter validation.
    Function {
        /// The function definition (name, description, parameter schema).
        function: Function,
    },

    /// Knowledge retrieval system access tools.
    ///
    /// Provides access to knowledge bases, document collections, or other
    /// structured information sources that the AI can query.
    Retrieval {
        /// The retrieval-tool descriptor.
        retrieval: Retrieval,
    },

    /// Web search capabilities for internet access.
    ///
    /// Enables the AI to perform web searches and access current information
    /// from the internet. Supports various search engines and configurations.
    WebSearch {
        /// The web-search-tool descriptor.
        web_search: WebSearch,
    },

    /// A provider-hosted MCP tool attached to a chat request.
    ///
    /// This config asks the chat service to connect to an MCP server. It is
    /// distinct from [`crate::mcp::McpClient`], which connects from this SDK.
    #[serde(rename = "mcp")]
    MCP {
        /// The MCP-tool descriptor.
        mcp: MCP,
    },
}

impl Validate for Tools {
    fn validate(&self) -> Result<(), validator::ValidationErrors> {
        match self {
            Self::Function { function } => function.validate(),
            Self::Retrieval { retrieval } => retrieval.validate(),
            Self::WebSearch { web_search } => web_search.validate(),
            Self::MCP { mcp } => mcp.validate(),
        }
    }
}

impl From<Function> for Tools {
    fn from(function: Function) -> Self {
        Self::Function { function }
    }
}

/// Definition of a caller-provided function that the model may invoke.
///
/// # Validation
///
/// * `name` - Must be 1 to 64 ASCII letters, digits, underscores, or hyphens
/// * `parameters` - Must be a valid JSON schema
#[derive(Clone, Serialize, Validate)]
#[validate(schema(function = "validate_function"))]
pub struct Function {
    /// The name of the function. Must match `[A-Za-z0-9_-]{1,64}`.
    #[validate(length(min = 1, max = 64))]
    pub name: String,

    /// A description of what the function does.
    pub description: String,

    /// JSON Schema object describing the function parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(custom(function = "validate_json_schema_value"))]
    pub parameters: Option<serde_json::Value>,
}

impl std::fmt::Debug for Function {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Function")
            .field("name", &self.name)
            .field("description", &"[REDACTED]")
            .field("parameters_configured", &self.parameters.is_some())
            .finish()
    }
}

fn validate_function(function: &Function) -> Result<(), validator::ValidationError> {
    if function.name.trim().is_empty() {
        return Err(validator::ValidationError::new("name_must_not_be_blank"));
    }
    if !function
        .name
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err(validator::ValidationError::new("invalid_function_name"));
    }
    if function
        .parameters
        .as_ref()
        .is_some_and(|parameters| !parameters.is_object())
    {
        return Err(validator::ValidationError::new(
            "function_parameters_must_be_an_object_schema",
        ));
    }
    Ok(())
}

impl Function {
    /// Create a function definition with a JSON Schema parameter object.
    ///
    /// # Examples
    ///
    /// ```
    /// use zai_rs::model::tools::Function;
    ///
    /// let func = Function::new(
    ///     "get_weather",
    ///     "Get current weather for a location",
    ///     serde_json::json!({
    ///         "type": "object",
    ///         "properties": { "location": { "type": "string" } }
    ///     })
    /// );
    /// ```
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: serde_json::Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters: Some(parameters),
        }
    }
}

/// Knowledge-base retrieval tool configuration.
///
/// Attaches a Zhipu AI knowledge base to a chat completion so the model can
/// retrieve relevant passages from it before answering. Create the knowledge
/// base (and obtain its `knowledge_id`) in the BigModel console, then pass the
/// id here via the [`Retrieval`] tool.
///
/// See the official guide:
/// <https://docs.bigmodel.cn/cn/guide/tools/knowledge/retrieval>.
///
/// # Wire form
///
/// Serializes as `{"knowledge_id":"…","prompt_template":"…"}` (the
/// `prompt_template` field is omitted when not set).
///
/// # Usage
///
/// ```rust
/// use zai_rs::model::tools::{Retrieval, Tools};
///
/// let tool = Tools::Retrieval {
///     retrieval: Retrieval::new("kb_1234567890"),
/// };
/// // Or attach a custom prompt template:
/// let tool = Tools::Retrieval {
///     retrieval: Retrieval::new("kb_1234567890")
///         .with_prompt_template("仅依据知识库回答：{knowledge}"),
/// };
/// ```
#[derive(Clone, Serialize, Validate)]
#[validate(schema(function = "validate_retrieval"))]
pub struct Retrieval {
    /// Knowledge-base id (required). Obtain it from the BigModel console after
    /// creating and populating a knowledge base.
    #[validate(length(min = 1))]
    pub knowledge_id: String,
    /// Optional prompt template applied when the model consumes retrieved
    /// knowledge. Serialized as `prompt_template`; omitted when `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 1))]
    pub prompt_template: Option<String>,
}

impl std::fmt::Debug for Retrieval {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Retrieval")
            .field("knowledge_id", &"[REDACTED]")
            .field(
                "prompt_template_configured",
                &self.prompt_template.is_some(),
            )
            .finish()
    }
}

fn validate_retrieval(retrieval: &Retrieval) -> Result<(), validator::ValidationError> {
    if retrieval.knowledge_id.trim().is_empty() {
        return Err(validator::ValidationError::new(
            "knowledge_id_must_not_be_blank",
        ));
    }
    if retrieval
        .prompt_template
        .as_deref()
        .is_some_and(|template| template.trim().is_empty())
    {
        return Err(validator::ValidationError::new(
            "prompt_template_must_not_be_blank",
        ));
    }
    Ok(())
}

impl Retrieval {
    /// Create a retrieval tool bound to `knowledge_id`.
    ///
    pub fn new(knowledge_id: impl Into<String>) -> Self {
        Self {
            knowledge_id: knowledge_id.into(),
            prompt_template: None,
        }
    }

    /// Attach a custom prompt template to the retrieval tool.
    pub fn with_prompt_template(mut self, prompt_template: impl Into<String>) -> Self {
        self.prompt_template = Some(prompt_template.into());
        self
    }
}

/// The order in which search results are returned.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResultSequence {
    /// Search results appear before the model's answer.
    Before,
    /// Search results appear after the model's answer.
    After,
}

/// Web-search configuration attached to a chat request.
#[derive(Clone, Serialize, Validate)]
#[validate(schema(function = "validate_web_search"))]
pub struct WebSearch {
    /// Search engine type (required). Supported: search_std, search_pro,
    /// search_pro_sogou, search_pro_quark.
    pub search_engine: SearchEngine,

    /// Whether to enable web search. Default is false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable: Option<bool>,

    /// Force-triggered search query string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_query: Option<String>,

    /// Whether to perform search intent detection. true: execute only when
    /// intent is detected; false: skip detection and search directly.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_intent: Option<bool>,

    /// Number of results to return (1-50).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 1, max = 50))]
    pub count: Option<u32>,

    /// Whitelist domain filter, e.g., "www.example.com".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_domain_filter: Option<String>,

    /// Time range filter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_recency_filter: Option<SearchRecencyFilter>,

    /// Snippet summary size: medium or high.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_size: Option<ContentSize>,

    /// Return sequence for search results: before or after.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_sequence: Option<ResultSequence>,

    /// Whether to include detailed search source information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_result: Option<bool>,

    /// Whether an answer requires search results to be returned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub require_search: Option<bool>,

    /// Custom prompt to post-process search results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_prompt: Option<String>,
}

impl std::fmt::Debug for WebSearch {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WebSearch")
            .field("search_engine", &self.search_engine)
            .field("enable", &self.enable)
            .field("search_query_configured", &self.search_query.is_some())
            .field("search_intent", &self.search_intent)
            .field("count", &self.count)
            .field(
                "search_domain_filter_configured",
                &self.search_domain_filter.is_some(),
            )
            .field("search_recency_filter", &self.search_recency_filter)
            .field("content_size", &self.content_size)
            .field("result_sequence", &self.result_sequence)
            .field("search_result", &self.search_result)
            .field("require_search", &self.require_search)
            .field("search_prompt_configured", &self.search_prompt.is_some())
            .finish()
    }
}

fn validate_web_search(web_search: &WebSearch) -> Result<(), validator::ValidationError> {
    if [
        web_search.search_query.as_deref(),
        web_search.search_domain_filter.as_deref(),
        web_search.search_prompt.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(|value| value.trim().is_empty())
    {
        Err(validator::ValidationError::new(
            "search_text_must_not_be_blank",
        ))
    } else {
        Ok(())
    }
}

impl WebSearch {
    /// Create a WebSearch config with the required search engine; other fields
    /// are optional.
    pub fn new(search_engine: SearchEngine) -> Self {
        Self {
            search_engine,
            enable: None,
            search_query: None,
            search_intent: None,
            count: None,
            search_domain_filter: None,
            search_recency_filter: None,
            content_size: None,
            result_sequence: None,
            search_result: None,
            require_search: None,
            search_prompt: None,
        }
    }

    /// Enable or disable web search.
    pub fn with_enable(mut self, enable: bool) -> Self {
        self.enable = Some(enable);
        self
    }
    /// Set a forced search query.
    pub fn with_search_query(mut self, query: impl Into<String>) -> Self {
        self.search_query = Some(query.into());
        self
    }
    /// Set search intent detection behavior.
    pub fn with_search_intent(mut self, search_intent: bool) -> Self {
        self.search_intent = Some(search_intent);
        self
    }
    /// Set results count (1-50).
    pub fn with_count(mut self, count: u32) -> Self {
        self.count = Some(count);
        self
    }
    /// Restrict to a whitelist domain.
    pub fn with_search_domain_filter(mut self, domain: impl Into<String>) -> Self {
        self.search_domain_filter = Some(domain.into());
        self
    }
    /// Set time range filter.
    pub fn with_search_recency_filter(mut self, filter: SearchRecencyFilter) -> Self {
        self.search_recency_filter = Some(filter);
        self
    }
    /// Set content size.
    pub fn with_content_size(mut self, size: ContentSize) -> Self {
        self.content_size = Some(size);
        self
    }
    /// Set result sequence.
    pub fn with_result_sequence(mut self, seq: ResultSequence) -> Self {
        self.result_sequence = Some(seq);
        self
    }
    /// Toggle returning detailed search source info.
    pub fn with_search_result(mut self, enable: bool) -> Self {
        self.search_result = Some(enable);
        self
    }
    /// Require search results for answering.
    pub fn with_require_search(mut self, require: bool) -> Self {
        self.require_search = Some(require);
        self
    }
    /// Set a custom prompt to post-process search results.
    pub fn with_search_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.search_prompt = Some(prompt.into());
        self
    }
}
/// Provider-side MCP connection configuration embedded in a chat request.
///
/// This does not create a local MCP connection. For Zhipu-hosted MCP servers,
/// put the MCP code in `server_label` and leave `server_url` unset. To call MCP
/// tools directly from this SDK, use [`crate::mcp::McpClient`].
#[derive(Clone, Serialize, Validate)]
#[validate(schema(function = "validate_mcp"))]
pub struct MCP {
    /// MCP server identifier (required). If connecting to Zhipu MCP via code,
    /// put the code here.
    #[validate(length(min = 1))]
    pub server_label: String,

    /// MCP server URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(url)]
    pub server_url: Option<String>,

    /// Transport type. Default: streamable-http.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport_type: Option<MCPTransportType>,

    /// Allowed tool names.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub allowed_tools: Vec<String>,

    /// Authentication headers required by the MCP server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<BTreeMap<String, String>>,
}

fn validate_mcp(mcp: &MCP) -> Result<(), validator::ValidationError> {
    if mcp.server_label.trim().is_empty() {
        return Err(validator::ValidationError::new(
            "server_label_must_not_be_blank",
        ));
    }
    if let Some(server_url) = mcp.server_url.as_deref() {
        let Ok(url) = server_url.parse::<url::Url>() else {
            return Err(validator::ValidationError::new("invalid_server_url"));
        };
        if !matches!(url.scheme(), "http" | "https")
            || !url.username().is_empty()
            || url.password().is_some()
            || url.fragment().is_some()
        {
            return Err(validator::ValidationError::new("invalid_server_url"));
        }
    }
    if mcp.allowed_tools.iter().any(|tool| tool.trim().is_empty()) {
        return Err(validator::ValidationError::new(
            "allowed_tool_must_not_be_blank",
        ));
    }
    if mcp.headers.as_ref().is_some_and(|headers| {
        headers.iter().any(|(name, value)| {
            !valid_forwarded_header_name(name) || !valid_forwarded_header_value(value)
        })
    }) {
        return Err(validator::ValidationError::new("invalid_forwarded_header"));
    }
    Ok(())
}

fn valid_forwarded_header_name(name: &str) -> bool {
    !name.is_empty()
        && name.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'!' | b'#'
                        | b'$'
                        | b'%'
                        | b'&'
                        | b'\''
                        | b'*'
                        | b'+'
                        | b'-'
                        | b'.'
                        | b'^'
                        | b'_'
                        | b'`'
                        | b'|'
                        | b'~'
                )
        })
}

fn valid_forwarded_header_value(value: &str) -> bool {
    !value.trim().is_empty()
        && value
            .chars()
            .all(|character| character == '\t' || !character.is_control())
}

impl std::fmt::Debug for MCP {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MCP")
            .field("server_label", &"[REDACTED]")
            .field("server_url_configured", &self.server_url.is_some())
            .field("transport_type", &self.transport_type)
            .field("allowed_tool_count", &self.allowed_tools.len())
            .field("header_count", &self.headers.as_ref().map(BTreeMap::len))
            .finish()
    }
}

impl MCP {
    /// Create a new MCP config with required server_label and default transport
    /// type.
    pub fn new(server_label: impl Into<String>) -> Self {
        Self {
            server_label: server_label.into(),
            server_url: None,
            transport_type: Some(MCPTransportType::StreamableHttp),
            allowed_tools: Vec::new(),
            headers: None,
        }
    }

    /// Set the MCP server URL.
    pub fn with_server_url(mut self, url: impl Into<String>) -> Self {
        self.server_url = Some(url.into());
        self
    }
    /// Set the MCP transport type.
    pub fn with_transport_type(mut self, transport: MCPTransportType) -> Self {
        self.transport_type = Some(transport);
        self
    }
    /// Replace the allowed tool list.
    pub fn with_allowed_tools(mut self, tools: impl Into<Vec<String>>) -> Self {
        self.allowed_tools = tools.into();
        self
    }
    /// Add a single allowed tool.
    pub fn add_allowed_tool(mut self, tool: impl Into<String>) -> Self {
        self.allowed_tools.push(tool.into());
        self
    }
    /// Set authentication headers map.
    pub fn with_headers(mut self, headers: BTreeMap<String, String>) -> Self {
        self.headers = Some(headers);
        self
    }
    /// Add or update a single header entry.
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers
            .get_or_insert_with(BTreeMap::new)
            .insert(key.into(), value.into());
        self
    }
}

/// Allowed MCP transport types.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum MCPTransportType {
    /// Server-Sent Events transport.
    Sse,
    /// Streamable HTTP transport.
    StreamableHttp,
}

/// Requested response representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum ResponseFormat {
    /// Plain text response format.
    Text,
    /// Structured JSON object response format.
    JsonObject,
}

/// Controls how the model selects tools during a chat completion.
///
/// This is the value carried by the `tool_choice` request parameter. It is only
/// meaningful when [`Tools`] are also attached to the request. The frozen API
/// schema accepts only the bare string `"auto"`; the older `"none"` and forced
/// function object forms were never part of this operation's contract.
///
/// # Usage
///
/// ```rust
/// use zai_rs::model::tools::ToolChoice;
///
/// // Let the model decide (default behaviour):
/// let choice = ToolChoice::auto();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolChoice {
    /// The model decides whether to call a tool (`"auto"` on the wire).
    Auto,
}

impl ToolChoice {
    /// Let the model decide whether to call a tool (`"auto"`).
    pub const fn auto() -> Self {
        Self::Auto
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ThinkingType tests
    #[test]
    fn test_thinking_type_enabled_serialization() {
        let thinking = ThinkingType::enabled();
        let json = serde_json::to_string(&thinking).unwrap();
        assert!(json.contains("\"type\":\"enabled\""));
        assert!(!json.contains("clear_thinking"));
    }

    #[test]
    fn test_thinking_type_disabled_serialization() {
        let thinking = ThinkingType::disabled();
        let json = serde_json::to_string(&thinking).unwrap();
        assert!(json.contains("\"type\":\"disabled\""));
        assert!(!json.contains("clear_thinking"));
    }

    #[test]
    fn test_thinking_type_with_clear_thinking_serialization() {
        let thinking = ThinkingType::enabled().with_clear_thinking(false);
        let json = serde_json::to_string(&thinking).unwrap();
        assert!(json.contains("\"type\":\"enabled\""));
        assert!(json.contains("\"clear_thinking\":false"));
    }

    #[test]
    fn test_thinking_type_disabled_with_clear_thinking() {
        let thinking = ThinkingType::disabled().with_clear_thinking(true);
        let json = serde_json::to_string(&thinking).unwrap();
        assert!(json.contains("\"type\":\"disabled\""));
        assert!(json.contains("\"clear_thinking\":true"));
    }

    // Function tests
    #[test]
    fn test_function_new() {
        let params = serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        });
        let func = Function::new("test_func", "A test function", params);

        assert_eq!(func.name, "test_func");
        assert_eq!(func.description, "A test function");
        assert!(func.parameters.is_some());
    }

    #[test]
    fn test_function_serialization() {
        let params = serde_json::json!({
            "type": "object",
            "properties": {
                "value": {"type": "number"}
            }
        });
        let func = Function::new("test_func", "A test function", params);
        let json = serde_json::to_string(&func).unwrap();

        assert!(json.contains("\"name\":\"test_func\""));
        assert!(json.contains("\"description\":\"A test function\""));
        assert!(json.contains("\"properties\""));
    }

    #[test]
    fn test_function_validation() {
        let params = serde_json::json!({
            "type": "object",
            "properties": {}
        });
        let func = Function::new("valid_name", "Description", params.clone());

        // Name length validation: 1-64 characters
        assert!(func.validate().is_ok());
        assert!(
            Function::new("valid-name", "Description", params.clone())
                .validate()
                .is_ok()
        );

        let invalid_name = Function::new("", "Description", params.clone());
        assert!(invalid_name.validate().is_err());
        let blank_name = Function::new("   ", "Description", params.clone());
        assert!(blank_name.validate().is_err());
        let punctuation = Function::new("invalid!", "Description", params.clone());
        assert!(punctuation.validate().is_err());

        let long_name = Function::new("a".repeat(65), "Description", params);
        assert!(long_name.validate().is_err());
    }

    // Retrieval tests
    #[test]
    fn test_retrieval_new() {
        let retrieval = Retrieval::new("kb_123").with_prompt_template("template");
        assert_eq!(retrieval.knowledge_id, "kb_123");
        assert_eq!(retrieval.prompt_template, Some("template".to_string()));
    }

    #[test]
    fn test_retrieval_new_without_template() {
        let retrieval = Retrieval::new("kb_456");
        assert_eq!(retrieval.knowledge_id, "kb_456");
        assert!(retrieval.prompt_template.is_none());
    }

    #[test]
    fn test_retrieval_serialization() {
        let retrieval = Retrieval::new("kb_789");
        let json = serde_json::to_string(&retrieval).unwrap();
        assert!(json.contains("\"knowledge_id\":\"kb_789\""));
        // prompt_template should be omitted when None
        assert!(!json.contains("prompt_template"));
    }

    #[test]
    fn retrieval_prompt_template_serializes() {
        let retrieval = Retrieval::new("kb_builder").with_prompt_template("ctx: {knowledge}");
        let json = serde_json::to_value(&retrieval).unwrap();
        assert_eq!(json["knowledge_id"], "kb_builder");
        assert_eq!(json["prompt_template"], "ctx: {knowledge}");
    }

    #[test]
    fn tool_validation_rejects_blank_optional_values() {
        assert!(Retrieval::new(" ").validate().is_err());
        assert!(
            WebSearch::new(SearchEngine::SearchPro)
                .with_search_query(" ")
                .validate()
                .is_err()
        );
        assert!(MCP::new(" ").validate().is_err());
        assert!(
            MCP::new("server")
                .with_server_url("ftp://example.com")
                .validate()
                .is_err()
        );
        assert!(
            MCP::new("server")
                .with_header("Authorization\r\nInjected", "secret")
                .validate()
                .is_err()
        );
    }

    // WebSearch tests
    #[test]
    fn test_web_search_new() {
        let web_search = WebSearch::new(SearchEngine::SearchPro);
        assert_eq!(web_search.search_engine, SearchEngine::SearchPro);
        assert!(web_search.enable.is_none());
    }

    #[test]
    fn test_web_search_with_enable() {
        let web_search = WebSearch::new(SearchEngine::SearchPro).with_enable(true);
        assert_eq!(web_search.enable, Some(true));
    }

    #[test]
    fn test_web_search_with_search_query() {
        let web_search = WebSearch::new(SearchEngine::SearchPro).with_search_query("test query");
        assert_eq!(web_search.search_query, Some("test query".to_string()));
    }

    #[test]
    fn test_web_search_with_search_intent() {
        let web_search = WebSearch::new(SearchEngine::SearchPro).with_search_intent(true);
        assert_eq!(web_search.search_intent, Some(true));
    }

    #[test]
    fn test_web_search_with_count() {
        let web_search = WebSearch::new(SearchEngine::SearchPro).with_count(10);
        assert_eq!(web_search.count, Some(10));
    }

    #[test]
    fn test_web_search_with_search_domain_filter() {
        let web_search =
            WebSearch::new(SearchEngine::SearchPro).with_search_domain_filter("example.com");
        assert_eq!(
            web_search.search_domain_filter,
            Some("example.com".to_string())
        );
    }

    #[test]
    fn test_web_search_with_search_recency_filter() {
        let filter = SearchRecencyFilter::OneDay;
        let web_search = WebSearch::new(SearchEngine::SearchPro).with_search_recency_filter(filter);
        assert_eq!(web_search.search_recency_filter, Some(filter));
    }

    #[test]
    fn test_web_search_with_content_size() {
        let size = ContentSize::Medium;
        let web_search = WebSearch::new(SearchEngine::SearchPro).with_content_size(size);
        assert_eq!(web_search.content_size, Some(size));
    }

    #[test]
    fn test_web_search_with_result_sequence() {
        let seq = ResultSequence::After;
        let web_search = WebSearch::new(SearchEngine::SearchPro).with_result_sequence(seq);
        assert_eq!(web_search.result_sequence, Some(seq));
    }

    #[test]
    fn test_web_search_with_search_result() {
        let web_search = WebSearch::new(SearchEngine::SearchPro).with_search_result(true);
        assert_eq!(web_search.search_result, Some(true));
    }

    #[test]
    fn test_web_search_with_require_search() {
        let web_search = WebSearch::new(SearchEngine::SearchPro).with_require_search(true);
        assert_eq!(web_search.require_search, Some(true));
    }

    #[test]
    fn test_web_search_with_search_prompt() {
        let web_search =
            WebSearch::new(SearchEngine::SearchPro).with_search_prompt("custom prompt");
        assert_eq!(web_search.search_prompt, Some("custom prompt".to_string()));
    }

    #[test]
    fn test_web_search_serialization() {
        let web_search = WebSearch::new(SearchEngine::SearchPro)
            .with_enable(true)
            .with_count(5);
        let json = serde_json::to_string(&web_search).unwrap();
        assert!(json.contains("\"search_engine\""));
        assert!(json.contains("\"enable\":true"));
        assert!(json.contains("\"count\":5"));
    }

    // MCP tests
    #[test]
    fn test_mcp_new() {
        let mcp = MCP::new("server_label");
        assert_eq!(mcp.server_label, "server_label");
        assert_eq!(mcp.transport_type, Some(MCPTransportType::StreamableHttp));
        assert!(mcp.allowed_tools.is_empty());
    }

    #[test]
    fn test_mcp_with_server_url() {
        let mcp = MCP::new("server_label").with_server_url("https://example.com");
        assert_eq!(mcp.server_url, Some("https://example.com".to_string()));
    }

    #[test]
    fn test_mcp_with_transport_type() {
        let mcp = MCP::new("server_label").with_transport_type(MCPTransportType::Sse);
        assert_eq!(mcp.transport_type, Some(MCPTransportType::Sse));
    }

    #[test]
    fn test_mcp_with_allowed_tools() {
        let mcp = MCP::new("server_label")
            .with_allowed_tools(vec!["tool1".to_string(), "tool2".to_string()]);
        assert_eq!(mcp.allowed_tools.len(), 2);
        assert!(mcp.allowed_tools.contains(&"tool1".to_string()));
    }

    #[test]
    fn test_mcp_add_allowed_tool() {
        let mcp = MCP::new("server_label")
            .add_allowed_tool("tool1")
            .add_allowed_tool("tool2");
        assert_eq!(mcp.allowed_tools.len(), 2);
    }

    #[test]
    fn test_mcp_with_headers() {
        let mut headers = BTreeMap::new();
        headers.insert("Authorization".to_string(), "Bearer token".to_string());
        let mcp = MCP::new("server_label").with_headers(headers.clone());
        assert_eq!(mcp.headers, Some(headers));
    }

    #[test]
    fn test_mcp_with_header() {
        let mcp = MCP::new("server_label").with_header("Authorization", "Bearer token");
        let debug = format!("{mcp:?}");
        assert!(!debug.contains("Bearer token"));
        assert!(!debug.contains("Authorization"));
        assert!(debug.contains("header_count"));
        let headers = mcp.headers.unwrap();
        assert_eq!(
            headers.get("Authorization"),
            Some(&"Bearer token".to_string())
        );
    }

    #[test]
    fn test_mcp_serialization() {
        let mcp = MCP::new("server_label")
            .with_server_url("https://example.com")
            .with_transport_type(MCPTransportType::Sse);
        let json = serde_json::to_string(&mcp).unwrap();
        assert!(json.contains("\"server_label\":\"server_label\""));
        assert!(json.contains("\"server_url\":\"https://example.com\""));
        assert!(json.contains("\"transport_type\":\"sse\""));
        // allowed_tools should be omitted when empty
        assert!(!json.contains("allowed_tools"));
    }

    #[test]
    fn mcp_validation_rejects_credentialed_or_fragmented_urls() {
        assert!(
            MCP::new("server")
                .with_server_url("https://user:secret@example.com/mcp")
                .validate()
                .is_err()
        );
        assert!(
            MCP::new("server")
                .with_server_url("https://example.com/mcp#secret")
                .validate()
                .is_err()
        );
    }

    #[test]
    fn tool_debug_output_redacts_caller_content_and_remote_configuration() {
        let function = Function::new(
            "lookup",
            "private-description",
            serde_json::json!({"private-schema-key": {"type": "string"}}),
        );
        let retrieval =
            Retrieval::new("private-knowledge").with_prompt_template("private-template");
        let search = WebSearch::new(SearchEngine::SearchPro)
            .with_search_query("private-query")
            .with_search_domain_filter("private.example")
            .with_search_prompt("private-search-prompt");
        let mcp = MCP::new("private-server")
            .with_server_url("https://private.example/mcp?token=private-token")
            .add_allowed_tool("private-tool")
            .with_header("Authorization", "private-header");
        let debug = format!("{function:?} {retrieval:?} {search:?} {mcp:?}");
        for secret in [
            "private-description",
            "private-schema-key",
            "private-knowledge",
            "private-template",
            "private-query",
            "private.example",
            "private-search-prompt",
            "private-server",
            "private-token",
            "private-tool",
            "Authorization",
            "private-header",
        ] {
            assert!(!debug.contains(secret), "Debug leaked {secret}");
        }
    }

    // MCPTransportType tests
    #[test]
    fn test_mcp_transport_type_sse_serialization() {
        let transport = MCPTransportType::Sse;
        let json = serde_json::to_string(&transport).unwrap();
        assert!(json.contains("\"sse\""));
    }

    #[test]
    fn test_mcp_transport_type_streamable_http_serialization() {
        let transport = MCPTransportType::StreamableHttp;
        let json = serde_json::to_string(&transport).unwrap();
        assert!(json.contains("\"streamable-http\""));
    }

    // ResponseFormat tests
    #[test]
    fn test_response_format_text_serialization() {
        let format = ResponseFormat::Text;
        let json = serde_json::to_string(&format).unwrap();
        assert!(json.contains("\"type\":\"text\""));
    }

    #[test]
    fn test_response_format_json_object_serialization() {
        let format = ResponseFormat::JsonObject;
        let json = serde_json::to_string(&format).unwrap();
        assert!(json.contains("\"type\":\"json_object\""));
    }

    // ToolChoice tests
    #[test]
    fn test_tool_choice_auto_serializes_as_bare_string() {
        let json = serde_json::to_value(ToolChoice::auto()).unwrap();
        assert_eq!(json, serde_json::json!("auto"));
    }

    // Tools enum tests
    #[test]
    fn test_tools_function_serialization() {
        let func = Function::new("test_func", "test", serde_json::json!({}));
        let tools = Tools::Function { function: func };
        let json = serde_json::to_string(&tools).unwrap();
        assert!(json.contains("\"type\":\"function\""));
        assert!(json.contains("\"name\":\"test_func\""));
    }

    #[test]
    fn test_tools_retrieval_serialization() {
        let retrieval = Retrieval::new("kb_123");
        let tools = Tools::Retrieval { retrieval };
        let json = serde_json::to_string(&tools).unwrap();
        assert!(json.contains("\"type\":\"retrieval\""));
        assert!(json.contains("\"knowledge_id\":\"kb_123\""));
    }

    #[test]
    fn test_tools_web_search_serialization() {
        let web_search = WebSearch::new(SearchEngine::SearchPro);
        let tools = Tools::WebSearch { web_search };
        let json = serde_json::to_string(&tools).unwrap();
        assert!(json.contains("\"type\":\"web_search\""));
        assert!(json.contains("\"search_engine\""));
    }

    #[test]
    fn test_tools_mcp_serialization() {
        let mcp = MCP::new("server_label");
        let tools = Tools::MCP { mcp };
        let json = serde_json::to_string(&tools).unwrap();
        assert!(json.contains("\"type\":\"mcp\""));
        assert!(json.contains("\"server_label\":\"server_label\""));
    }

    // ResultSequence tests
    #[test]
    fn test_result_sequence_before_serialization() {
        let seq = ResultSequence::Before;
        let json = serde_json::to_string(&seq).unwrap();
        assert!(json.contains("\"before\""));
    }

    #[test]
    fn test_result_sequence_after_serialization() {
        let seq = ResultSequence::After;
        let json = serde_json::to_string(&seq).unwrap();
        assert!(json.contains("\"after\""));
    }
}
