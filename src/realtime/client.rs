//! [`RealtimeClient`] — entry point for the realtime API.

use std::sync::Arc;

use super::session::SessionBuilder;
use crate::client::endpoint::EndpointConfig;

/// Authentication mode for the realtime WebSocket handshake.
#[derive(Debug, Clone)]
pub enum AuthMode {
    /// Server-side Bearer auth: `Authorization: Bearer {API_KEY}` (default).
    Bearer,
    /// JWT auth using a short-lived token derived locally from the API key.
    /// Only the derived token is sent in the WebSocket handshake. Keeping the
    /// original key away from an untrusted browser or device remains the
    /// application's responsibility.
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
/// ```no_run
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
            endpoint_config: EndpointConfig::defaults()
                .unwrap_or_else(|_| EndpointConfig::builder().build(false).unwrap()),
        }
    }

    /// Switch to JWT auth, signing tokens valid for `ttl_seconds`.
    ///
    /// The value is validated when the session is built and must be between one
    /// second and seven days, inclusive.
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
        let leaked: &'static str = Box::leak(base.into().into_boxed_str());
        self.endpoint_config = EndpointConfig::builder()
            .realtime(leaked)
            .build(false)
            .unwrap();
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
        self.endpoint_config
            .resolve_route(crate::client::routes::REALTIME_CONNECT, &[])
            .unwrap_or_default()
    }

    /// Current auth mode.
    pub fn auth(&self) -> &AuthMode {
        &self.auth
    }

    /// A reference to the configured API key.
    ///
    /// Treat this value as sensitive and never include it in logs or errors.
    pub fn api_key(&self) -> &str {
        &self.api_key
    }
}
