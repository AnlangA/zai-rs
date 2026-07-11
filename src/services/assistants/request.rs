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
        let route = crate::client::routes::ASSISTANTS_INVOKE;
        let url = client.endpoints().resolve_route(route, &[])?;
        client
            .send_json::<_, AssistantInvokeResponse>(route.method(), url, &self.body)
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
        let route = crate::client::routes::ASSISTANTS_LIST;
        let url = client.endpoints().resolve_route(route, &[])?;
        client
            .send_json::<_, AssistantListResponse>(route.method(), url, &self.body)
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
        let route = crate::client::routes::ASSISTANTS_CONVERSATIONS;
        let url = client.endpoints().resolve_route(route, &[])?;
        client
            .send_json::<_, AssistantConversationListResponse>(route.method(), url, &self.body)
            .await
    }
}

impl Default for AssistantConversationListRequest {
    fn default() -> Self {
        Self::new()
    }
}
