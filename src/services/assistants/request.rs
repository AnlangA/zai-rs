//! Request types for assistant invocation and discovery endpoints.

use crate::ZaiResult;
use crate::client::ZaiClient;

use super::response::{
    AssistantConversationListResponse, AssistantInvokeResponse, AssistantListResponse,
};

/// Invoke an assistant with an open-schema JSON body.
pub struct AssistantInvokeRequest {
    body: serde_json::Value,
}

impl AssistantInvokeRequest {
    /// Create an assistant invocation request.
    pub fn new(body: serde_json::Value) -> Self {
        Self { body }
    }

    /// Send the request through `client`.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<AssistantInvokeResponse> {
        let route = crate::client::routes::ASSISTANTS_INVOKE;
        let url = client.endpoints().resolve_route(route, &[])?;
        client
            .send_json::<_, AssistantInvokeResponse>(route.method(), url, &self.body)
            .await
    }
}

/// List the assistants available to the current account.
pub struct AssistantListRequest {
    body: serde_json::Value,
}

impl AssistantListRequest {
    /// Create an assistant-list request.
    pub fn new() -> Self {
        Self {
            body: serde_json::json!({}),
        }
    }

    /// Send the request through `client`.
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

/// List assistant conversations available to the current account.
pub struct AssistantConversationListRequest {
    body: serde_json::Value,
}

impl AssistantConversationListRequest {
    /// Create an assistant-conversation-list request.
    pub fn new() -> Self {
        Self {
            body: serde_json::json!({}),
        }
    }

    /// Send the request through `client`.
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
