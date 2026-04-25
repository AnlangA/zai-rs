//! # Agent API Module
//!
//! This module provides support for Zhipu AI's Agent API, which enables
//! advanced AI agent interactions including multi-turn conversations, tool
//! use, and persistent state management.
//!
//! ## Features
//!
//! - Agent creation and management
//! - Multi-turn conversations
//! - Tool and function calling
//! - Async task management
//! - Conversation history

use serde::{Deserialize, Serialize};

use crate::client::http::{HttpClientConfig, http_client_with_config, parse_api_error_response};

pub mod request;
pub mod response;

pub use request::*;
pub use response::*;

/// Agent API endpoint for creating and managing AI agents
pub const AGENT_API_URL: &str = "https://open.bigmodel.cn/api/paas/v4/agents";

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
    base_url: String,
    http_config: HttpClientConfig,
    client: reqwest::Client,
}

impl AgentClient {
    /// Create a new Agent API client
    pub fn new(api_key: impl Into<String>) -> Self {
        let config = HttpClientConfig::default();
        let client = http_client_with_config(&config);
        Self {
            api_key: api_key.into(),
            base_url: AGENT_API_URL.to_string(),
            http_config: config,
            client,
        }
    }

    /// Create a new agent with custom base URL
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Set custom HTTP client configuration (timeout, retries, etc.)
    pub fn with_http_config(mut self, config: HttpClientConfig) -> Self {
        self.client = http_client_with_config(&config);
        self.http_config = config;
        self
    }

    /// Create a new AI agent
    pub async fn create_agent(
        &self,
        request: AgentCreateRequest,
    ) -> crate::ZaiResult<AgentCreateResponse> {
        self.send_request(&self.base_url, &request).await
    }

    /// Get agent details by ID
    pub async fn get_agent(&self, agent_id: &str) -> crate::ZaiResult<AgentDetails> {
        let url = format!("{}/{}", self.base_url, agent_id);
        self.send_get_request(&url).await
    }

    /// Update an existing agent
    pub async fn update_agent(
        &self,
        agent_id: &str,
        request: AgentUpdateRequest,
    ) -> crate::ZaiResult<AgentUpdateResponse> {
        let url = format!("{}/{}", self.base_url, agent_id);
        self.send_request(&url, &request).await
    }

    /// Delete an agent
    pub async fn delete_agent(&self, agent_id: &str) -> crate::ZaiResult<AgentDeleteResponse> {
        let url = format!("{}/{}", self.base_url, agent_id);
        let response = self
            .client
            .delete(&url)
            .bearer_auth(&self.api_key)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            Err(parse_api_error_response(status, body))
        }
    }

    /// Send a chat message to an agent
    pub async fn chat(
        &self,
        agent_id: &str,
        request: AgentChatRequest,
    ) -> crate::ZaiResult<AgentChatResponse> {
        let url = format!("{}/{}/chat", self.base_url, agent_id);
        self.send_request(&url, &request).await
    }

    /// Get conversation history
    pub async fn get_history(
        &self,
        agent_id: &str,
        limit: Option<u32>,
    ) -> crate::ZaiResult<ConversationHistory> {
        let mut url = format!("{}/{}/history", self.base_url, agent_id);
        if let Some(l) = limit {
            url.push_str(&format!("?limit={}", l));
        }
        self.send_get_request(&url).await
    }

    /// Internal method to send POST requests (reuses connection pool)
    async fn send_request<T: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
        body: &T,
    ) -> crate::ZaiResult<R> {
        let response = self
            .client
            .post(url)
            .bearer_auth(&self.api_key)
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            Err(parse_api_error_response(status, body))
        }
    }

    /// Internal method to send GET requests (reuses connection pool)
    async fn send_get_request<R: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
    ) -> crate::ZaiResult<R> {
        let response = self
            .client
            .get(url)
            .bearer_auth(&self.api_key)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            Err(parse_api_error_response(status, body))
        }
    }
}
