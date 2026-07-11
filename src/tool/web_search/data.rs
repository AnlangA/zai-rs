use std::sync::Arc;

use crate::{
    ZaiResult,
    client::ZaiClient,
    client::http::{HttpClientConfig, parse_typed_response, send_json_request},
    tool::web_search::{request::*, response::*},
};

/// Web search API request builder (P05: routes through [`ZaiClient`]).
pub struct WebSearchRequest {
    body: WebSearchBody,
}

impl WebSearchRequest {
    pub fn new(search_query: String, search_engine: SearchEngine) -> Self {
        Self {
            body: WebSearchBody::new(search_query, search_engine),
        }
    }

    pub fn with_body(body: WebSearchBody) -> Self {
        Self { body }
    }

    pub fn with_search_intent(mut self, enabled: bool) -> Self {
        self.body = self.body.with_search_intent(enabled);
        self
    }
    pub fn with_count(mut self, count: i32) -> Self {
        self.body = self.body.with_count(count);
        self
    }
    pub fn with_domain_filter(mut self, domain: impl Into<String>) -> Self {
        self.body = self.body.with_domain_filter(domain.into());
        self
    }
    pub fn with_recency_filter(mut self, filter: SearchRecencyFilter) -> Self {
        self.body = self.body.with_recency_filter(filter);
        self
    }
    pub fn with_content_size(mut self, size: ContentSize) -> Self {
        self.body = self.body.with_content_size(size);
        self
    }
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.body = self.body.with_request_id(request_id.into());
        self
    }
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.body = self.body.with_user_id(user_id.into());
        self
    }

    pub fn validate(&self) -> ZaiResult<()> {
        self.body.validate_constraints()
    }

    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<WebSearchResponse> {
        self.validate()?;
        let url = client
            .endpoints()
            .resolve(crate::client::ApiFamily::PaasV4, &["web_search"])?;
        let config = transport_config_from_client(client);
        let resp = send_json_request(
            reqwest::Method::POST,
            url,
            client.secret().expose(),
            &self.body,
            Arc::new(config),
        )
        .await?;
        parse_typed_response::<WebSearchResponse>(resp).await
    }
}

fn transport_config_from_client(client: &ZaiClient) -> HttpClientConfig {
    let t = client.transport();
    HttpClientConfig {
        timeout: std::time::Duration::from_secs(t.request_timeout.as_secs()),
        max_retries: u32::from(t.max_attempts).saturating_sub(1),
        enable_compression: t.enable_compression,
        retry_delay: crate::client::http::RetryDelay::Exponential {
            base: std::time::Duration::from_millis(500),
            max: std::time::Duration::from_secs(5),
        },
        enable_logging: false,
        mask_sensitive_data: true,
    }
}
