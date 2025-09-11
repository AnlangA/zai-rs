//! Tool definitions and configurations for the model API.
//!
//! This module defines the various tools that can be used by the assistant,
//! including function calling, retrieval systems, web search, and MCP tools.

use super::model_validate::validate_json_schema_value;
use serde::Serialize;
use std::collections::HashMap;
use validator::*;

/// Controls thinking/reasoning capabilities in AI models.
///
/// This enum determines whether a model should engage in step-by-step reasoning
/// when processing requests. Thinking mode can improve accuracy for complex tasks
/// but may increase response time and token usage.
///
/// ## Variants
///
/// - `Enabled` - Model performs explicit reasoning steps before responding
/// - `Disabled` - Model responds directly without showing reasoning process
///
/// ## Usage
///
/// ```rust,ignore
/// let client = ChatCompletion::new(model, messages, api_key)
///     .with_thinking(ThinkingType::Enabled);
/// ```
///
/// ## Model Compatibility
///
/// Thinking capabilities are available only on models that implement the
/// `ThinkEnable` trait, such as GLM-4.5 series models.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type")]
pub enum ThinkingType {
    /// Enable thinking capabilities for enhanced reasoning.
    ///
    /// When enabled, the model will show its reasoning process step-by-step,
    /// which can improve accuracy for complex logical or analytical tasks.
    Enabled,

    /// Disable thinking capabilities for direct responses.
    ///
    /// When disabled, the model responds directly without showing intermediate
    /// reasoning steps, resulting in faster responses and lower token usage.
    Disabled,
}

/// Available tools that AI assistants can invoke during conversations.
///
/// This enum defines the different categories of external tools and capabilities
/// that can be made available to AI models. Each tool type serves specific purposes
/// and has its own configuration requirements.
///
/// ## Tool Categories
///
/// ### Function Tools
/// Custom user-defined functions that the AI can call with structured parameters.
/// Useful for integrating external APIs, databases, or business logic.
///
/// ### Retrieval Tools
/// Access to knowledge bases, document collections, or information retrieval systems.
/// Enables the AI to query structured knowledge sources.
///
/// ### Web Search Tools
/// Internet search capabilities for accessing current information.
/// Allows the AI to perform web searches and retrieve up-to-date information.
///
/// ### MCP Tools
/// Model Context Protocol tools for standardized tool integration.
/// Provides a standardized interface for tool communication.
///
/// ## Usage
///
/// ```rust,ignore
/// // Function tool
/// let function_tool = Tools::Function {
///     function: Function::new("get_weather", "Get weather data", parameters)
/// };
///
/// // Web search tool
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
    /// Allows the AI to invoke user-defined functions with structured arguments.
    /// Functions must be pre-defined with JSON schemas for parameter validation.
    Function { function: Function },

    /// Knowledge retrieval system access tools.
    ///
    /// Provides access to knowledge bases, document collections, or other
    /// structured information sources that the AI can query.
    Retrieval { retrieval: Retrieval },

    /// Web search capabilities for internet access.
    ///
    /// Enables the AI to perform web searches and access current information
    /// from the internet. Supports various search engines and configurations.
    WebSearch { web_search: WebSearch },

    /// Model Context Protocol (MCP) tools.
    ///
    /// Standardized tools that follow the Model Context Protocol specification,
    /// providing a consistent interface for tool integration and communication.
    MCP { mcp: MCP },
}

/// Definition of a callable function tool.
///
/// This structure defines a function that can be called by the assistant,
/// including its name, description, and parameter schema.
///
/// # Validation
///
/// * `name` - Must be between 1 and 64 characters
/// * `parameters` - Must be a valid JSON schema
#[derive(Debug, Clone, Serialize, Validate)]
pub struct Function {
    /// The name of the function. Must be between 1 and 64 characters.
    #[validate(length(min = 1, max = 64))]
    pub name: String,

    /// A description of what the function does.
    pub description: String,

    /// JSON schema describing the function's parameters.
    /// Server expects an object; keep as Value to avoid double-encoding strings.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(custom(function = "validate_json_schema_value"))]
    pub parameters: Option<serde_json::Value>,
}

impl Function {
    /// Creates a new function call definition.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the function
    /// * `description` - A description of what the function does
    /// * `parameters` - JSON schema string describing the function parameters
    ///
    /// # Returns
    ///
    /// A new `Function` instance.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let func = Function::new(
    ///     "get_weather",
    ///     "Get current weather for a location",
    ///     r#"{"type": "object", "properties": {"location": {"type": "string"}}}"#
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

/// Configuration for retrieval tool capabilities.
///
/// This structure represents a retrieval tool that can access knowledge bases
/// or document collections. Currently a placeholder for future expansion.
#[derive(Debug, Clone, Serialize)]
pub struct Retrieval {
    knowledge_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt_template: Option<String>,
}

impl Retrieval {
    /// Creates a new `Retrieval` instance.
    pub fn new(knowledge_id: impl Into<String>, prompt_template: Option<String>) -> Self {
        Self {
            knowledge_id: knowledge_id.into(),
            prompt_template,
        }
    }
}

/// Configuration for web search tool capabilities.
///
/// This structure represents a web search tool that can perform internet searches.
/// Fields mirror the external web_search schema.
#[derive(Debug, Clone, Serialize, Validate)]
pub struct WebSearch {
    /// Search engine type (required). Supported: search_std, search_pro, search_pro_sogou, search_pro_quark.
    pub search_engine: SearchEngine,

    /// Whether to enable web search. Default is false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable: Option<bool>,

    /// Force-triggered search query string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_query: Option<String>,

    /// Whether to perform search intent detection. true: execute only when intent is detected; false: skip detection and search directly.
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

impl WebSearch {
    /// Create a WebSearch config with the required search engine; other fields are optional.
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

/// Supported search engines.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchEngine {
    SearchStd,
    SearchPro,
    SearchProSogou,
    SearchProQuark,
}

/// Search time range filter.
#[derive(Debug, Clone, Serialize)]
pub enum SearchRecencyFilter {
    #[serde(rename = "oneDay")]
    OneDay,
    #[serde(rename = "oneWeek")]
    OneWeek,
    #[serde(rename = "oneMonth")]
    OneMonth,
    #[serde(rename = "oneYear")]
    OneYear,
    #[serde(rename = "noLimit")]
    NoLimit,
}

/// Search snippet size.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentSize {
    Medium,
    High,
}

/// The order in which search results are returned.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultSequence {
    Before,
    After,
}

/// Configuration for Model Context Protocol (MCP) tools.
///
/// Represents the MCP connection configuration. When connecting to Zhipu's MCP server
/// using an MCP code, fill `server_label` with that code and leave `server_url` empty.
#[derive(Debug, Clone, Serialize, Validate)]
pub struct MCP {
    /// MCP server identifier (required). If connecting to Zhipu MCP via code, put the code here.
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
    pub headers: Option<HashMap<String, String>>,
}

impl MCP {
    /// Create a new MCP config with required server_label and default transport type.
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
    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = Some(headers);
        self
    }
    /// Add or update a single header entry.
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let mut map = self.headers.unwrap_or_default();
        map.insert(key.into(), value.into());
        self.headers = Some(map);
        self
    }
}

/// Allowed MCP transport types.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MCPTransportType {
    Sse,
    StreamableHttp,
}

/// Specifies the format for the model's response.
///
/// This enum controls how the model should structure its output, either as
/// plain text or as a structured JSON object.
///
/// # Variants
///
/// * `Text` - Plain text response format
/// * `JsonObject` - Structured JSON object response format
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum ResponseFormat {
    /// Plain text response format.
    Text,
    /// Structured JSON object response format.
    JsonObject,
}
