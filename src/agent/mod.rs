//! # Agent API Module
//!
//! Provides support for Zhipu AI's Agent API, enabling advanced AI agent
//! interactions including creation, multi-turn conversations, tool use,
//! and persistent state management.
//!
//! # Core Types
//!
//! - [`AgentClient`] — Main client for all agent operations
//! - [`AgentCreateRequest`] — Request body for creating an agent
//! - [`AgentChatRequest`] — Request body for sending a chat message
//!
//! # Usage
//!
//! ```rust,ignore
//! use zai_rs::agent::{AgentClient, AgentCreateRequest};
//!
//! let client = AgentClient::new(api_key);
//!
//! // Create an agent
//! let agent = client.create_agent(request).await?;
//!
//! // Chat with the agent
//! let response = client.chat(&agent.id, chat_request).await?;
//!
//! // Retrieve conversation history
//! let history = client.get_history(&agent.id, Some(10)).await?;
//! ```

use serde::{Deserialize, Serialize};

use crate::client::{
    endpoints::{ApiBase, EndpointConfig, build_query, join_url, paths},
    http::{HttpClientConfig, parse_typed_response, send_empty_request, send_json_request},
};

pub mod request;
pub mod response;

pub use request::*;
pub use response::*;

/// Agent API path for creating and managing AI agents.
pub const AGENT_API_PATH: &str = paths::AGENTS;

/// Agent client for managing AI agent interactions
///
/// # Example
///
/// ```rust,ignore
/// use zai_rs::agent::{AgentClient, AgentCreateRequest};
///
/// let client = AgentClient::new(api_key);
/// let request = AgentCreateRequest::builder()
///     .name("My Assistant")
///     .description("A helpful assistant")
///     .build();
///
/// let agent = client.create_agent(request).await?;
/// ```
pub struct AgentClient {
    api_key: String,
    endpoint_config: EndpointConfig,
    api_base: ApiBase,
    http_config: HttpClientConfig,
}

impl AgentClient {
    /// Create a new Agent API client
    pub fn new(api_key: impl Into<String>) -> Self {
        let config = HttpClientConfig::default();
        Self {
            api_key: api_key.into(),
            endpoint_config: EndpointConfig::default(),
            api_base: ApiBase::PaasV4,
            http_config: config,
        }
    }

    /// Create a new agent with custom base URL
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.api_base = ApiBase::Custom(base_url.into());
        self
    }

    /// Replace the full [`EndpointConfig`] used to resolve URLs.
    pub fn with_endpoint_config(mut self, endpoint_config: EndpointConfig) -> Self {
        self.endpoint_config = endpoint_config;
        self
    }

    /// Set custom HTTP client configuration (timeout, retries, etc.)
    pub fn with_http_config(mut self, config: HttpClientConfig) -> Self {
        self.http_config = config;
        self
    }

    fn url(&self, path: &str) -> String {
        self.endpoint_config.url(&self.api_base, path)
    }

    /// Create a new AI agent
    pub async fn create_agent(
        &self,
        request: AgentCreateRequest,
    ) -> crate::ZaiResult<AgentCreateResponse> {
        self.send_request(&self.url(paths::AGENTS), &request).await
    }

    /// Get agent details by ID
    pub async fn get_agent(&self, agent_id: &str) -> crate::ZaiResult<AgentDetails> {
        let url = self.url(&join_url(paths::AGENTS, agent_id));
        self.send_get_request(&url).await
    }

    /// Update an existing agent
    pub async fn update_agent(
        &self,
        agent_id: &str,
        request: AgentUpdateRequest,
    ) -> crate::ZaiResult<AgentUpdateResponse> {
        let url = self.url(&join_url(paths::AGENTS, agent_id));
        self.send_request(&url, &request).await
    }

    /// Delete an agent
    pub async fn delete_agent(&self, agent_id: &str) -> crate::ZaiResult<AgentDeleteResponse> {
        let url = self.url(&join_url(paths::AGENTS, agent_id));
        let response = send_empty_request(
            reqwest::Method::DELETE,
            url,
            &self.api_key,
            std::sync::Arc::new(self.http_config.clone()),
        )
        .await?;
        parse_typed_response::<AgentDeleteResponse>(response).await
    }

    /// Send a chat message to an agent
    pub async fn chat(
        &self,
        agent_id: &str,
        request: AgentChatRequest,
    ) -> crate::ZaiResult<AgentChatResponse> {
        let url = self.url(&join_url(&join_url(paths::AGENTS, agent_id), "chat"));
        self.send_request(&url, &request).await
    }

    /// Get conversation history
    pub async fn get_history(
        &self,
        agent_id: &str,
        limit: Option<u32>,
    ) -> crate::ZaiResult<ConversationHistory> {
        let base = self.url(&join_url(&join_url(paths::AGENTS, agent_id), "history"));
        let url = match limit {
            Some(l) => build_query(&base, [("limit", l.to_string())]),
            None => base,
        };
        self.send_get_request(&url).await
    }

    /// Internal method to send POST requests (reuses connection pool)
    async fn send_request<T: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
        body: &T,
    ) -> crate::ZaiResult<R> {
        let response = send_json_request(
            reqwest::Method::POST,
            url.to_string(),
            &self.api_key,
            body,
            std::sync::Arc::new(self.http_config.clone()),
        )
        .await?;
        parse_typed_response::<R>(response).await
    }

    /// Internal method to send GET requests (reuses connection pool)
    async fn send_get_request<R: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
    ) -> crate::ZaiResult<R> {
        let response = send_empty_request(
            reqwest::Method::GET,
            url.to_string(),
            &self.api_key,
            std::sync::Arc::new(self.http_config.clone()),
        )
        .await?;
        parse_typed_response::<R>(response).await
    }
}
