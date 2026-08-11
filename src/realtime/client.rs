//! [`RealtimeClient`] — entry point for the realtime API.

use std::sync::Arc;

use super::{config::RealtimeTransportConfig, session::SessionBuilder};
use crate::{
    ZaiResult,
    client::{ApiFamily, endpoint::EndpointConfig, secret::ApiSecret},
};

/// Authentication mode for the realtime WebSocket handshake.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
/// [`RealtimeClient::with_jwt`], optionally set a validated
/// [`RealtimeTransportConfig`], then start a
/// session with [`RealtimeClient::session`].
///
/// ```rust,no_run
/// use zai_rs::{model::GLM_realtime_flash, realtime::RealtimeClient};
///
/// # async fn go(key: String) -> zai_rs::ZaiResult<()> {
/// let session = RealtimeClient::new(key)
///     .session(GLM_realtime_flash {})
///     .build()
///     .await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct RealtimeClient {
    api_key: Arc<ApiSecret>,
    auth: AuthMode,
    endpoint_config: EndpointConfig,
    transport_config: RealtimeTransportConfig,
}

impl RealtimeClient {
    /// Create a realtime client using Bearer auth and the default endpoints.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: Arc::new(ApiSecret::new(api_key)),
            auth: AuthMode::Bearer,
            endpoint_config: EndpointConfig::defaults()
                .expect("built-in realtime endpoint must be valid"),
            transport_config: RealtimeTransportConfig::default(),
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

    /// Use `config` as the default transport/session policy for subsequently
    /// created session builders.
    ///
    /// A builder snapshots this value when [`Self::session`] is called. An
    /// override applied directly to that builder takes precedence and does not
    /// affect this client or any other session.
    pub fn with_transport_config(mut self, config: RealtimeTransportConfig) -> Self {
        self.transport_config = config;
        self
    }

    /// Override only the realtime base URL (`wss://...`).
    ///
    /// Only the realtime family is replaced; all other endpoint overrides are
    /// preserved. Invalid or insecure public URLs return an error.
    pub fn try_with_realtime_base(mut self, base: impl AsRef<str>) -> ZaiResult<Self> {
        self.endpoint_config =
            self.endpoint_config
                .with_base(ApiFamily::Realtime, base.as_ref(), false)?;
        Ok(self)
    }

    /// Begin building a realtime session for `model`.
    ///
    /// The model bound is checked at compile time via [`super::RealtimeModel`].
    pub fn session<M: super::RealtimeModel>(&self, model: M) -> SessionBuilder {
        let realtime_url = self.realtime_url();
        let model_name: String = model.into();
        SessionBuilder::new(
            Arc::clone(&self.api_key),
            self.auth,
            realtime_url,
            model_name,
            self.transport_config.clone(),
        )
    }

    /// The resolved realtime WebSocket URL.
    pub fn realtime_url(&self) -> String {
        self.endpoint_config.base(ApiFamily::Realtime).to_string()
    }

    /// Current auth mode.
    pub fn auth(&self) -> &AuthMode {
        &self.auth
    }

    /// Default transport/session policy snapshotted by new session builders.
    pub fn transport_config(&self) -> &RealtimeTransportConfig {
        &self.transport_config
    }
}

impl std::fmt::Debug for RealtimeClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RealtimeClient")
            .field("api_key", &self.api_key)
            .field("auth", &self.auth)
            .field("endpoint_config", &self.endpoint_config)
            .field("transport_config", &self.transport_config)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_redacts_api_key() {
        let client = RealtimeClient::new("abcdefghij.0123456789abcdef");
        let debug = format!("{client:?}");
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("abcdefghij"));
    }

    #[test]
    fn realtime_override_accepts_an_owned_non_static_base() {
        let base = format!("wss://{}/realtime", "example.com");
        let client = RealtimeClient::new("abcdefghij.0123456789abcdef")
            .try_with_realtime_base(&base)
            .unwrap();
        assert_eq!(client.realtime_url(), "wss://example.com/realtime");
        assert!(
            client
                .try_with_realtime_base("ws://public.example/realtime")
                .is_err()
        );
    }
}
