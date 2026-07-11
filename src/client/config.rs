//! [`ZaiClient`], [`ZaiClientBuilder`] and [`HttpTransportConfig`] (plan P02).
//!
//! A `ZaiClient` is the single shared entry point: it owns an `Arc<ClientInner>`
//! holding the secret, validated endpoints, the one `reqwest::Client`, transport
//! policies and the [`Services`] facades. `Clone` is cheap (one `Arc` bump) and
//! does NOT copy the config, secret or connection pool.
//!
//! The builder only accepts an [`HttpTransportConfig`] and the API key — it
//! never takes a pre-built `reqwest::Client` (plan P02.9). Insecure transport
//! is opt-in via [`ZaiClientBuilder::allow_insecure_transport`]; HTTP/WS bases
//! must still be loopback.

use std::sync::Arc;
use std::time::Duration;

use crate::ZaiResult;
use crate::client::endpoint::EndpointConfig;
use crate::client::secret::ApiSecret;
use crate::client::services::Services;

/// The shared 0.5 client.
///
/// Construct via [`ZaiClient::builder`] or [`ZaiClient::from_env`]. Cloning a
/// `ZaiClient` shares the underlying connection pool, secret and config — it
/// does not duplicate them (plan P02.3).
#[derive(Clone)]
pub struct ZaiClient {
    inner: Arc<ClientInner>,
}

/// Interior of a [`ZaiClient`], shared via `Arc`.
pub(crate) struct ClientInner {
    /// The API key, wrapped so it never appears in Debug/Display.
    pub(crate) secret: ApiSecret,
    /// Validated per-family base URLs.
    pub(crate) endpoints: EndpointConfig,
    /// Transport policy.
    pub(crate) transport: HttpTransportConfig,
    /// The single reqwest client built by the SDK.
    #[allow(dead_code)]
    pub(crate) reqwest: reqwest::Client,
    pub(crate) sender: crate::client::transport::Transport,
}

impl ZaiClient {
    /// Start a builder that requires an API key.
    pub fn builder(api_key: impl Into<String>) -> ZaiClientBuilder {
        ZaiClientBuilder {
            api_key: api_key.into(),
            endpoints: EndpointConfig::builder(),
            transport: HttpTransportConfig::default(),
            allow_insecure: false,
        }
    }

    /// Read `ZHIPU_API_KEY` from the environment and build with defaults.
    pub fn from_env() -> ZaiResult<Self> {
        let key = std::env::var("ZHIPU_API_KEY").map_err(|_| crate::ZaiError::ApiError {
            code: crate::client::error::codes::SDK_CONFIG,
            message: "ZHIPU_API_KEY environment variable not set".to_string(),
        })?;
        if key.trim().is_empty() {
            return Err(crate::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_CONFIG,
                message: "ZHIPU_API_KEY environment variable is empty".to_string(),
            });
        }
        Self::builder(key).build()
    }

    /// Obtain the service facades (`client.services().chat()`,
    /// `client.services().files()`, …). `Services` is a zero-sized handle that
    /// re-borrows this client, so obtaining it is free.
    pub fn services(&self) -> Services {
        Services::new(self.clone())
    }

    /// Borrow the validated endpoints.
    pub fn endpoints(&self) -> &EndpointConfig {
        &self.inner.endpoints
    }

    /// Borrow the transport policy.
    pub fn transport(&self) -> &HttpTransportConfig {
        &self.inner.transport
    }

    /// Borrow the shared reqwest client (crate-private use only).
    #[allow(dead_code)]
    pub(crate) fn reqwest(&self) -> &reqwest::Client {
        &self.inner.reqwest
    }

    /// Borrow the API secret (crate-private; only for Authorization construction).
    pub(crate) fn secret(&self) -> &ApiSecret {
        &self.inner.secret
    }

    /// Send a JSON request through the unified transport and decode its body.
    pub(crate) async fn send_json<B, R>(
        &self,
        method: &'static str,
        url: String,
        body: &B,
    ) -> ZaiResult<R>
    where
        B: serde::Serialize + ?Sized,
        R: serde::de::DeserializeOwned,
    {
        let bytes = bytes::Bytes::from(serde_json::to_vec(body).map_err(crate::ZaiError::from)?);
        let request = crate::client::transport::request::PreparedRequest {
            method,
            url,
            body: crate::client::transport::request::BodyKind::Bytes(&bytes),
            retry_safety: crate::client::transport::retry::RetrySafety::for_method(method),
            retry_override: None,
            response_mode: crate::client::transport::request::ResponseMode::Json,
            route_template: "typed-api-request",
        };
        self.inner.sender.send(&request).await?.json()
    }

    /// Send a body-less request through the unified transport and decode JSON.
    pub(crate) async fn send_empty<R>(&self, method: &'static str, url: String) -> ZaiResult<R>
    where
        R: serde::de::DeserializeOwned,
    {
        let request = crate::client::transport::request::PreparedRequest {
            method,
            url,
            body: crate::client::transport::request::BodyKind::None,
            retry_safety: crate::client::transport::retry::RetrySafety::for_method(method),
            retry_override: None,
            response_mode: crate::client::transport::request::ResponseMode::Json,
            route_template: "typed-api-request",
        };
        self.inner.sender.send(&request).await?.json()
    }

    /// Send a JSON request whose successful response is binary.
    pub(crate) async fn send_json_bytes<B: serde::Serialize + ?Sized>(
        &self,
        method: &'static str,
        url: String,
        body: &B,
    ) -> ZaiResult<bytes::Bytes> {
        let bytes = bytes::Bytes::from(serde_json::to_vec(body).map_err(crate::ZaiError::from)?);
        let request = crate::client::transport::request::PreparedRequest {
            method,
            url,
            body: crate::client::transport::request::BodyKind::Bytes(&bytes),
            retry_safety: crate::client::transport::retry::RetrySafety::for_method(method),
            retry_override: None,
            response_mode: crate::client::transport::request::ResponseMode::Binary,
            route_template: "binary-api-request",
        };
        self.inner.sender.send(&request).await?.bytes()
    }

    /// Send a body-less request whose successful response is binary.
    pub(crate) async fn send_empty_bytes(
        &self,
        method: &'static str,
        url: String,
    ) -> ZaiResult<bytes::Bytes> {
        let request = crate::client::transport::request::PreparedRequest {
            method,
            url,
            body: crate::client::transport::request::BodyKind::None,
            retry_safety: crate::client::transport::retry::RetrySafety::for_method(method),
            retry_override: None,
            response_mode: crate::client::transport::request::ResponseMode::Binary,
            route_template: "binary-api-request",
        };
        self.inner.sender.send(&request).await?.bytes()
    }

    /// Send a multipart request through the unified transport and decode JSON.
    pub(crate) async fn send_multipart<R: serde::de::DeserializeOwned>(
        &self,
        method: &'static str,
        url: String,
        factory: &crate::client::transport::multipart::MultipartBodyFactory,
    ) -> ZaiResult<R> {
        let request = crate::client::transport::request::PreparedRequest {
            method,
            url,
            body: crate::client::transport::request::BodyKind::Multipart(factory),
            retry_safety: crate::client::transport::retry::RetrySafety::for_method(method),
            retry_override: None,
            response_mode: crate::client::transport::request::ResponseMode::Json,
            route_template: "multipart-api-request",
        };
        self.inner.sender.send(&request).await?.json()
    }
}

impl std::fmt::Debug for ZaiClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never exposes the secret; the inner secret's own Debug is [REDACTED].
        f.debug_struct("ZaiClient")
            .field("secret", &self.inner.secret)
            .field("endpoints", &self.inner.endpoints)
            .field("transport", &self.inner.transport)
            .finish_non_exhaustive()
    }
}

// `ClientInner` needs to hold `Services`, which borrows `&ClientInner` — break
// the cycle by initializing `Services` after the inner is pinned. We store it
// in an `OnceLock`-free slot by constructing it in `build()` after the Arc is
// created. To keep the borrow valid for the lifetime of the Arc, `Services` is
// a ZST handle that only re-borrows the `Arc<ClientInner>` it was made from; it
// carries no data of its own. See `services.rs`.

/// Builder for [`ZaiClient`].
pub struct ZaiClientBuilder {
    api_key: String,
    endpoints: crate::client::endpoint::EndpointConfigBuilder,
    transport: HttpTransportConfig,
    allow_insecure: bool,
}

impl ZaiClientBuilder {
    /// Override a family base URL. The value must be a `&'static str` (an
    /// official/known base); arbitrary `String` overrides are intentionally not
    /// accepted here to keep configuration auditable.
    pub fn endpoint(
        mut self,
        family: crate::client::endpoint::ApiFamily,
        base: &'static str,
    ) -> Self {
        use crate::client::endpoint::ApiFamily::*;
        match family {
            PaasV4 => self.endpoints = self.endpoints.paas_v4(base),
            CodingPaasV4 => self.endpoints = self.endpoints.coding_paas_v4(base),
            AgentV1 => self.endpoints = self.endpoints.agent_v1(base),
            LlmApplication | ApplicationV2 | ApplicationV3 => {
                self.endpoints = self.endpoints.llm_application(base)
            },
            Zrag => self.endpoints = self.endpoints.zrag(base),
            Monitor => self.endpoints = self.endpoints.monitor(base),
            Realtime => self.endpoints = self.endpoints.realtime(base),
        }
        self
    }

    /// Replace the transport policy.
    pub fn transport(mut self, transport: HttpTransportConfig) -> Self {
        self.transport = transport;
        self
    }

    /// Permit HTTP/WS bases, but only for loopback/localhost hosts (plan P02.7).
    pub fn allow_insecure_transport(mut self, allow: bool) -> Self {
        self.allow_insecure = allow;
        self
    }

    /// Finalize. Rejects empty/blank keys; validates every endpoint URL.
    pub fn build(self) -> ZaiResult<ZaiClient> {
        if self.api_key.trim().is_empty() {
            return Err(crate::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_CONFIG,
                message: "ZaiClient requires a non-empty api_key".to_string(),
            });
        }
        let endpoints = self.endpoints.build(self.allow_insecure)?;
        let reqwest = build_reqwest_client(&self.transport)?;
        let secret = ApiSecret::new(self.api_key);
        let sender = crate::client::transport::Transport::new(
            reqwest.clone(),
            secret.clone(),
            &self.transport,
        );
        let inner = Arc::new(ClientInner {
            secret,
            endpoints,
            transport: self.transport,
            reqwest,
            sender,
        });
        Ok(ZaiClient { inner })
    }
}

/// Construct the single `reqwest::Client` for a transport policy.
///
/// Plan P02.10: fixed connection-pool sizing, the SDK-controlled headers
/// (Authorization/Accept/Content-Type/User-Agent) are set per-request, not here.
fn build_reqwest_client(transport: &HttpTransportConfig) -> ZaiResult<reqwest::Client> {
    let mut builder = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .pool_max_idle_per_host(8)
        .pool_idle_timeout(Some(Duration::from_secs(90)))
        .tcp_keepalive(Some(Duration::from_secs(60)))
        .connect_timeout(transport.connect_timeout)
        .timeout(transport.request_timeout);
    if transport.enable_compression {
        builder = builder.gzip(true);
    }
    builder.build().map_err(crate::ZaiError::from)
}

// --- HttpTransportConfig ---------------------------------------------------

/// Allow-listed names for user-supplied additional headers (plan P02.9).
const ALLOWED_HEADER_NAMES: &[&str] = &["Accept-Language", "X-Correlation-ID", "X-Test-Client"];

/// A single allow-listed additional header.
#[derive(Debug, Clone)]
pub struct AdditionalHeader {
    name: &'static str,
    value: String,
}

impl AdditionalHeader {
    /// Construct an allow-listed header. Returns `Err` for disallowed names or
    /// over-long / non-printable values (>1024 bytes).
    pub fn new(name: &str, value: &str) -> ZaiResult<Self> {
        let allowed = ALLOWED_HEADER_NAMES.contains(&name);
        if !allowed {
            return Err(crate::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_CONFIG,
                message: format!("header name {name:?} is not allow-listed"),
            });
        }
        if value.len() > 1024 {
            return Err(crate::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_CONFIG,
                message: "additional header value exceeds 1024 bytes".to_string(),
            });
        }
        if !value.chars().all(|c| !c.is_control()) {
            return Err(crate::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_CONFIG,
                message: "additional header value must be printable".to_string(),
            });
        }
        // Leak-resolved at build time into a &'static str keyed off the
        // allow-list (the set is tiny and fixed).
        let static_name = ALLOWED_HEADER_NAMES
            .iter()
            .copied()
            .find(|n| *n == name)
            .expect("validated above");
        Ok(Self {
            name: static_name,
            value: value.to_string(),
        })
    }

    pub fn name(&self) -> &'static str {
        self.name
    }
    pub fn value(&self) -> &str {
        &self.value
    }
}

/// Per-request retry-safety override (plan §4). The only safe escape hatch; does
/// NOT enter the serialized request body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryOverride {
    /// Treat this request as idempotent for retry purposes (e.g. a POST whose
    /// server-side effect is known to be idempotent).
    AssumeIdempotent,
}

/// Transport policy for [`ZaiClient`].
///
/// Limits are upper bounds: the builder only ever lets callers *lower* them
/// (plan P02.9). Defaults match plan §4.
#[derive(Debug, Clone)]
pub struct HttpTransportConfig {
    /// Connect timeout (default 10s).
    pub connect_timeout: Duration,
    /// Per-attempt request timeout (default 60s).
    pub request_timeout: Duration,
    /// Whether to advertise gzip (default true).
    pub enable_compression: bool,
    /// Maximum retry attempts, inclusive of the first attempt (default 3; the
    /// builder only allows lowering to 1 or 2).
    pub max_attempts: u8,
    /// Allow-listed additional headers attached to every request.
    pub additional_headers: Vec<AdditionalHeader>,
}

impl Default for HttpTransportConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(60),
            enable_compression: true,
            max_attempts: 3,
            additional_headers: Vec::new(),
        }
    }
}

impl HttpTransportConfig {
    /// Start a builder.
    pub fn builder() -> HttpTransportConfigBuilder {
        HttpTransportConfigBuilder {
            config: Self::default(),
        }
    }

    /// Lower the per-attempt request timeout. Values above the default are
    /// rejected (plan: builder only allows tightening).
    pub fn with_request_timeout(mut self, d: Duration) -> ZaiResult<Self> {
        if d > Duration::from_secs(60) {
            return Err(crate::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_CONFIG,
                message: "request_timeout may only be lowered (max 60s)".to_string(),
            });
        }
        self.request_timeout = d;
        Ok(self)
    }

    /// Lower the connect timeout (max 10s).
    pub fn with_connect_timeout(mut self, d: Duration) -> ZaiResult<Self> {
        if d > Duration::from_secs(10) {
            return Err(crate::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_CONFIG,
                message: "connect_timeout may only be lowered (max 10s)".to_string(),
            });
        }
        self.connect_timeout = d;
        Ok(self)
    }

    /// Lower the max attempts to 1 or 2 (the only values below the default 3).
    pub fn with_max_attempts(mut self, n: u8) -> ZaiResult<Self> {
        if n == 0 || n > 3 {
            return Err(crate::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_CONFIG,
                message: "max_attempts must be 1, 2 or 3".to_string(),
            });
        }
        self.max_attempts = n;
        Ok(self)
    }

    /// Add an allow-listed additional header.
    pub fn with_additional_header(mut self, header: AdditionalHeader) -> Self {
        self.additional_headers.push(header);
        self
    }
}

/// Builder for [`HttpTransportConfig`] (tighten-only).
pub struct HttpTransportConfigBuilder {
    config: HttpTransportConfig,
}

impl HttpTransportConfigBuilder {
    pub fn request_timeout(mut self, d: Duration) -> ZaiResult<Self> {
        self.config = self.config.with_request_timeout(d)?;
        Ok(self)
    }
    pub fn connect_timeout(mut self, d: Duration) -> ZaiResult<Self> {
        self.config = self.config.with_connect_timeout(d)?;
        Ok(self)
    }
    pub fn max_attempts(mut self, n: u8) -> ZaiResult<Self> {
        self.config = self.config.with_max_attempts(n)?;
        Ok(self)
    }
    pub fn additional_header(mut self, header: AdditionalHeader) -> Self {
        self.config.additional_headers.push(header);
        self
    }
    pub fn build(self) -> HttpTransportConfig {
        self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_rejects_blank_key() {
        assert!(ZaiClient::builder("   ").build().is_err());
        assert!(ZaiClient::builder("").build().is_err());
    }

    #[test]
    fn clone_shares_inner_no_secret_leak() {
        let c = ZaiClient::builder("abcdefghij.0123456789abcdef")
            .build()
            .unwrap();
        let c2 = c.clone();
        // Cloning produces a handle to the same inner; Debug stays redacted.
        let dbg = format!("{c2:?}");
        assert!(dbg.contains("[REDACTED]"));
        assert!(!dbg.contains("abcdefghij"));
    }

    #[test]
    fn additional_header_allow_list() {
        assert!(AdditionalHeader::new("X-Test-Client", "preserved").is_ok());
        assert!(AdditionalHeader::new("Authorization", "nope").is_err());
        assert!(AdditionalHeader::new("Cookie", "nope").is_err());
        assert!(AdditionalHeader::new("Proxy-Authorization", "nope").is_err());
    }

    #[test]
    fn additional_header_value_limits() {
        let long = "x".repeat(1025);
        assert!(AdditionalHeader::new("X-Test-Client", &long).is_err());
        assert!(AdditionalHeader::new("X-Test-Client", "ok\0bad").is_err());
    }

    #[test]
    fn transport_only_tightens() {
        // Request timeout above default rejected.
        assert!(
            HttpTransportConfig::default()
                .with_request_timeout(Duration::from_secs(120))
                .is_err()
        );
        // Lowering accepted.
        assert!(
            HttpTransportConfig::default()
                .with_request_timeout(Duration::from_secs(5))
                .is_ok()
        );
        // max_attempts only 1/2/3.
        assert!(HttpTransportConfig::default().with_max_attempts(0).is_err());
        assert!(HttpTransportConfig::default().with_max_attempts(4).is_err());
        assert!(HttpTransportConfig::default().with_max_attempts(2).is_ok());
    }
}
