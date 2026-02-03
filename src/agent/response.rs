//! Agent API response types

use serde::{Deserialize, Serialize};

/// Response from creating a new agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCreateResponse {
    /// The created agent ID
    pub id: String,

    /// Agent name
    pub name: String,

    /// Agent description
    pub description: Option<String>,

    /// Creation timestamp
    pub created_at: Option<u64>,

    /// Agent model
    pub model: Option<String>,
}

/// Detailed agent information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDetails {
    /// Agent ID
    pub id: String,

    /// Agent name
    pub name: String,

    /// Agent description
    pub description: Option<String>,

    /// System prompt
    pub system_prompt: Option<String>,

    /// Model used
    pub model: Option<String>,

    /// Available tools
    pub tools: Option<Vec<serde_json::Value>>,

    /// Agent configuration
    pub config: Option<AgentConfigResponse>,

    /// Creation timestamp
    pub created_at: Option<u64>,

    /// Last update timestamp
    pub updated_at: Option<u64>,
}

/// Agent configuration response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfigResponse {
    /// Temperature setting
    pub temperature: Option<f32>,

    /// Max tokens setting
    pub max_tokens: Option<u32>,

    /// Whether thinking is enabled
    pub thinking_enabled: Option<bool>,
}

/// Response from updating an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentUpdateResponse {
    /// Agent ID
    pub id: String,

    /// Success status
    pub success: bool,

    /// Update timestamp
    pub updated_at: Option<u64>,
}

/// Response from deleting an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDeleteResponse {
    /// Deleted agent ID
    pub id: String,

    /// Success status
    pub success: bool,

    /// Deletion timestamp
    pub deleted_at: Option<u64>,
}

/// Response from agent chat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentChatResponse {
    /// Conversation ID
    pub conversation_id: Option<String>,

    /// Session ID
    pub session_id: Option<String>],

    /// Agent response
    pub response: AgentMessage,

    /// Tool calls made by the agent
    pub tool_calls: Option<Vec<AgentToolCall>>,

    /// Usage statistics
    pub usage: Option<AgentUsage>,
}

/// Agent message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    /// Message role (assistant/user)
    pub role: String,

    /// Message content
    pub content: String,

    /// Reasoning content (for thinking mode)
    pub reasoning_content: Option<String>,

    /// Timestamp
    pub timestamp: Option<u64>,
}

/// Tool call made by agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolCall {
    /// Tool call ID
    pub id: String,

    /// Tool name
    pub name: String,

    /// Tool arguments (JSON string)
    pub arguments: String,

    /// Tool result
    pub result: Option<serde_json::Value>,
}

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentUsage {
    /// Input tokens
    pub prompt_tokens: Option<u32>,

    /// Output tokens
    pub completion_tokens: Option<u32>,

    /// Total tokens
    pub total_tokens: Option<u32>,
}

/// Conversation history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationHistory {
    /// Conversation ID
    pub conversation_id: String,

    /// Messages in the conversation
    pub messages: Vec<AgentMessage>,

    /// Total number of messages
    pub total_count: Option<u32>,

    /// Whether there are more messages
    pub has_more: Option<bool>,
}
