use crate::ZaiResult;
use crate::client::http::{HttpClientConfig, RetryDelay, parse_typed_response, send_json_request};
use crate::client::v2::ZaiClient;

use super::response::{
    AssistantConversationListResponse, AssistantInvokeResponse, AssistantListResponse,
};

fn transport_config(client: &crate::client::v2::ZaiClient) -> HttpClientConfig {
    let t = client.transport();
    HttpClientConfig {
        timeout: std::time::Duration::from_secs(t.request_timeout.as_secs()),
        max_retries: u32::from(t.max_attempts).saturating_sub(1),
        enable_compression: t.enable_compression,
        retry_delay: RetryDelay::Exponential {
            base: std::time::Duration::from_millis(500),
            max: std::time::Duration::from_secs(5),
        },
        enable_logging: false,
        mask_sensitive_data: true,
    }
}

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
            .resolve(crate::client::v2::ApiFamily::PaasV4, &["assistant"])?;
        let config = transport_config(client);
        let resp = send_json_request(
            reqwest::Method::POST,
            url,
            client.secret().expose(),
            &self.body,
            std::sync::Arc::new(config),
        )
        .await?;
        parse_typed_response::<AssistantInvokeResponse>(resp).await
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
            .resolve(crate::client::v2::ApiFamily::PaasV4, &["assistant", "list"])?;
        let config = transport_config(client);
        let resp = send_json_request(
            reqwest::Method::POST,
            url,
            client.secret().expose(),
            &self.body,
            std::sync::Arc::new(config),
        )
        .await?;
        parse_typed_response::<AssistantListResponse>(resp).await
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
            crate::client::v2::ApiFamily::PaasV4,
            &["assistant", "conversation", "list"],
        )?;
        let config = transport_config(client);
        let resp = send_json_request(
            reqwest::Method::POST,
            url,
            client.secret().expose(),
            &self.body,
            std::sync::Arc::new(config),
        )
        .await?;
        parse_typed_response::<AssistantConversationListResponse>(resp).await
    }
}

impl Default for AssistantConversationListRequest {
    fn default() -> Self {
        Self::new()
    }
}
