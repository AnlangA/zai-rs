//! [`RealtimeClient`] — entry point for the realtime API.

use std::sync::Arc;

use super::session::SessionBuilder;
use crate::client::endpoints::{ApiBase, EndpointConfig};

/// Authentication mode for the realtime WebSocket handshake.
#[derive(Debug, Clone)]
pub enum AuthMode {
    /// Server-side Bearer auth: `Authorization: Bearer {API_KEY}` (default).
    Bearer,
    /// Client-side JWT auth: a short-lived token signed from the API key's
    /// secret, so the secret never leaves the server. Used when the WebSocket
    /// is opened directly from a browser/device.
    Jwt {
        /// Token validity in seconds.
        ttl_seconds: i64,
    },
}

/// Entry point for the realtime API.
///
/// Construct with [`RealtimeClient::new`], optionally switch to JWT auth via
/// [`RealtimeClient::with_jwt`], then start a session with
/// [`RealtimeClient::session`].
///
/// ```rust,no_run
/// use zai_rs::{model::GLM4_voice, realtime::RealtimeClient};
///
/// # async fn go(key: String) -> zai_rs::ZaiResult<()> {
/// let session = RealtimeClient::new(key)
///     .session(GLM4_voice {})
///     .build()
///     .await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct RealtimeClient {
    api_key: Arc<String>,
    auth: AuthMode,
    endpoint_config: EndpointConfig,
}

impl RealtimeClient {
    /// Create a realtime client using Bearer auth and the default endpoints.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: Arc::new(api_key.into()),
            auth: AuthMode::Bearer,
            endpoint_config: EndpointConfig::default(),
        }
    }

    /// Switch to client-side JWT auth, signing tokens valid for `ttl_seconds`.
    pub fn with_jwt(mut self, ttl_seconds: i64) -> Self {
        self.auth = AuthMode::Jwt { ttl_seconds };
        self
    }

    /// Switch (back) to server-side Bearer auth.
    pub fn with_bearer(mut self) -> Self {
        self.auth = AuthMode::Bearer;
        self
    }

    /// Override the full endpoint config (base URLs).
    pub fn with_endpoint_config(mut self, config: EndpointConfig) -> Self {
        self.endpoint_config = config;
        self
    }

    /// Override only the realtime base URL (`wss://...`).
    pub fn with_realtime_base(mut self, base: impl Into<String>) -> Self {
        self.endpoint_config = self.endpoint_config.with_realtime_base(base);
        self
    }

    /// Begin building a realtime session for `model`.
    ///
    /// The model bound is checked at compile time via [`super::RealtimeModel`].
    pub fn session<M: super::RealtimeModel>(&self, model: M) -> SessionBuilder {
        let realtime_url = self.realtime_url();
        let model_name: String = model.into();
        SessionBuilder::new(
            Arc::clone(&self.api_key),
            self.auth.clone(),
            realtime_url,
            model_name,
        )
    }

    /// The resolved realtime WebSocket URL.
    pub fn realtime_url(&self) -> String {
        self.endpoint_config.url(&ApiBase::Realtime, "")
    }

    /// Current auth mode.
    pub fn auth(&self) -> &AuthMode {
        &self.auth
    }

    /// A reference to the configured API key.
    pub fn api_key(&self) -> &str {
        &self.api_key
    }
}
