use crate::ZaiResult;
use crate::client::ZaiClient;

use super::response::{
    AssistantConversationListResponse, AssistantInvokeResponse, AssistantListResponse,
};

// ── AssistantInvokeRequest ──────────────────────────────────────────────

pub struct AssistantInvokeRequest {
    body: serde_json::Value,
}

impl AssistantInvokeRequest {
    pub fn new(body: serde_json::Value) -> Self {
        Self { body }
    }

    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<AssistantInvokeResponse> {
        let url = client
            .endpoints()
            .resolve(crate::client::ApiFamily::PaasV4, &["assistant"])?;
        client
            .send_json::<_, AssistantInvokeResponse>("POST", url, &self.body)
            .await
    }
}

// ── AssistantListRequest ────────────────────────────────────────────────

pub struct AssistantListRequest {
    body: serde_json::Value,
}

impl AssistantListRequest {
    pub fn new() -> Self {
        Self {
            body: serde_json::json!({}),
        }
    }

    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<AssistantListResponse> {
        let url = client
            .endpoints()
            .resolve(crate::client::ApiFamily::PaasV4, &["assistant", "list"])?;
        client
            .send_json::<_, AssistantListResponse>("POST", url, &self.body)
            .await
    }
}

impl Default for AssistantListRequest {
    fn default() -> Self {
        Self::new()
    }
}

// ── AssistantConversationListRequest ────────────────────────────────────

pub struct AssistantConversationListRequest {
    body: serde_json::Value,
}

impl AssistantConversationListRequest {
    pub fn new() -> Self {
        Self {
            body: serde_json::json!({}),
        }
    }

    pub async fn send_via(
        &self,
        client: &ZaiClient,
    ) -> ZaiResult<AssistantConversationListResponse> {
        let url = client.endpoints().resolve(
            crate::client::ApiFamily::PaasV4,
            &["assistant", "conversation", "list"],
        )?;
        client
            .send_json::<_, AssistantConversationListResponse>("POST", url, &self.body)
            .await
    }
}

impl Default for AssistantConversationListRequest {
    fn default() -> Self {
        Self::new()
    }
}
