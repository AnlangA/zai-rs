use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AssistantInvokeResponse {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub choices: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssistantListResponse {
    #[serde(default)]
    pub data: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssistantConversationListResponse {
    #[serde(default)]
    pub data: Vec<serde_json::Value>,
}
