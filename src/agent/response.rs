//! Agent API response types

use serde::{Deserialize, Serialize};

use super::request::AgentConfig;

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
    pub config: Option<AgentConfig>,

    /// Creation timestamp
    pub created_at: Option<u64>,

    /// Last update timestamp
    pub updated_at: Option<u64>,
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
    pub session_id: Option<String>,

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_create_response_serde_roundtrip() {
        let resp = AgentCreateResponse {
            id: "agent_123".to_string(),
            name: "TestAgent".to_string(),
            description: Some("A test agent".to_string()),
            created_at: Some(1700000000),
            model: Some("glm-4.5".to_string()),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: AgentCreateResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "agent_123");
        assert_eq!(parsed.name, "TestAgent");
        assert_eq!(parsed.description, Some("A test agent".to_string()));
    }

    #[test]
    fn test_agent_details_serde_roundtrip() {
        let details = AgentDetails {
            id: "agent_456".to_string(),
            name: "DetailAgent".to_string(),
            description: None,
            system_prompt: Some("Be helpful".to_string()),
            model: Some("glm-4.5-flash".to_string()),
            tools: None,
            config: None,
            created_at: Some(1700000000),
            updated_at: Some(1700000100),
        };
        let json = serde_json::to_string(&details).unwrap();
        let parsed: AgentDetails = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "agent_456");
        assert!(parsed.description.is_none());
    }

    #[test]
    fn test_agent_chat_response_serde_roundtrip() {
        let resp = AgentChatResponse {
            conversation_id: Some("conv_789".to_string()),
            session_id: Some("sess_012".to_string()),
            response: AgentMessage {
                role: "assistant".to_string(),
                content: "Hello!".to_string(),
                reasoning_content: None,
                timestamp: Some(1700000000),
            },
            tool_calls: None,
            usage: Some(AgentUsage {
                prompt_tokens: Some(10),
                completion_tokens: Some(5),
                total_tokens: Some(15),
            }),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: AgentChatResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.response.content, "Hello!");
        assert_eq!(parsed.usage.unwrap().total_tokens, Some(15));
    }

    #[test]
    fn test_agent_update_response_serde_roundtrip() {
        let resp = AgentUpdateResponse {
            id: "agent_123".to_string(),
            success: true,
            updated_at: Some(1700000200),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: AgentUpdateResponse = serde_json::from_str(&json).unwrap();
        assert!(parsed.success);
    }

    #[test]
    fn test_agent_delete_response_serde_roundtrip() {
        let resp = AgentDeleteResponse {
            id: "agent_123".to_string(),
            success: true,
            deleted_at: Some(1700000300),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: AgentDeleteResponse = serde_json::from_str(&json).unwrap();
        assert!(parsed.success);
    }

    #[test]
    fn test_conversation_history_serde_roundtrip() {
        let history = ConversationHistory {
            conversation_id: "conv_abc".to_string(),
            messages: vec![AgentMessage {
                role: "user".to_string(),
                content: "Hi".to_string(),
                reasoning_content: None,
                timestamp: None,
            }],
            total_count: Some(1),
            has_more: Some(false),
        };
        let json = serde_json::to_string(&history).unwrap();
        let parsed: ConversationHistory = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.messages.len(), 1);
        assert_eq!(parsed.messages[0].role, "user");
    }
}
