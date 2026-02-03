//! Agent API request types

use serde::{Deserialize, Serialize};
use validator::Validate;

/// Request to create a new AI agent
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct AgentCreateRequest {
    /// Agent name (required)
    #[validate(length(min = 1, max = 100))]
    pub name: String,

    /// Agent description (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(max = 1000))]
    pub description: Option<String>],

    /// System prompt for the agent (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,

    /// Model to use (optional, defaults to glm-4.5)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Tools available to the agent (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<serde_json::Value>>,

    /// Agent configuration (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<AgentConfig>,
}

impl AgentCreateRequest {
    /// Create a builder for AgentCreateRequest
    pub fn builder() -> AgentCreateRequestBuilder {
        AgentCreateRequestBuilder::default()
    }
}

/// Builder for creating AgentCreateRequest
#[derive(Default)]
pub struct AgentCreateRequestBuilder {
    name: Option<String>,
    description: Option<String>,
    system_prompt: Option<String>,
    model: Option<String>,
    tools: Option<Vec<serde_json::Value>>,
    config: Option<AgentConfig>,
}

impl AgentCreateRequestBuilder {
    /// Set the agent name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the agent description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the system prompt
    pub fn system_prompt(mut self, system_prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(system_prompt.into());
        self
    }

    /// Set the model
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Add tools to the agent
    pub fn tools(mut self, tools: Vec<serde_json::Value>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Set the agent configuration
    pub fn config(mut self, config: AgentConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Build the AgentCreateRequest
    pub fn build(self) -> Result<AgentCreateRequest, String> {
        let name = self.name.ok_or("name is required")?;

        Ok(AgentCreateRequest {
            name,
            description: self.description,
            system_prompt: self.system_prompt,
            model: self.model,
            tools: self.tools,
            config: self.config,
        })
    }
}

/// Agent configuration options
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct AgentConfig {
    /// Temperature for responses (0.0 to 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 0.0, max = 1.0))]
    pub temperature: Option<f32>,

    /// Maximum tokens in response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// Enable thinking mode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_enabled: Option<bool>,
}

/// Request to update an existing agent
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct AgentUpdateRequest {
    /// New agent name (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 1, max = 100))]
    pub name: Option<String>,

    /// New description (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(max = 1000))]
    pub description: Option<String>,

    /// New system prompt (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,

    /// New model (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// New tools (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<serde_json::Value>>,

    /// New configuration (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<AgentConfig>,
}

/// Request to chat with an agent
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct AgentChatRequest {
    /// User message
    #[validate(length(min = 1))]
    pub message: String,

    /// Conversation ID for multi-turn conversations (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,

    /// Session ID for tracking (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,

    /// Stream response (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// Additional parameters (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<AgentChatParameters>,
}

/// Additional chat parameters
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct AgentChatParameters {
    /// Temperature override
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 0.0, max = 1.0))]
    pub temperature: Option<f32>,

    /// Max tokens override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// Enable thinking for this request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_enabled: Option<bool>,
}
