//! Response types for assistant invocation and discovery endpoints.

use serde::Deserialize;

/// Response from invoking an assistant.
#[derive(Debug, Clone, Deserialize)]
pub struct AssistantInvokeResponse {
    /// Invocation identifier, when returned by the service.
    #[serde(default)]
    pub id: Option<String>,
    /// Assistant output choices in their upstream open-schema form.
    #[serde(default)]
    pub choices: Vec<serde_json::Value>,
}

/// Response containing assistants available to the current account.
#[derive(Debug, Clone, Deserialize)]
pub struct AssistantListResponse {
    /// Assistant records in their upstream open-schema form.
    #[serde(default)]
    pub data: Vec<serde_json::Value>,
}

/// Response containing assistant conversations.
#[derive(Debug, Clone, Deserialize)]
pub struct AssistantConversationListResponse {
    /// Conversation records in their upstream open-schema form.
    #[serde(default)]
    pub data: Vec<serde_json::Value>,
}
