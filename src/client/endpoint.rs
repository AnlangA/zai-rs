//! Validated, URL-based endpoint configuration.
//!
//! Each family base is stored as a parsed [`url::Url`]. Building rejects
//! relative URLs, userinfo, query strings and fragments; the scheme is checked
//! against the family (HTTPS/WSS by default, HTTP/WS only when insecure
//! transport is explicitly allowed and the host passes a syntactic local-host
//! check). Dynamic
//! path segments go through [`EndpointConfig::resolve`], which
//! percent-encodes via `url::PathSegmentsMut` and rejects empty / `.` / `..`
//! segments — never raw string concatenation.

use url::Url;

use super::routes::{Route, Segment};
use crate::{ZaiError, ZaiResult, client::error::codes};

/// API endpoint families understood by [`EndpointConfig`].
///
/// Each variant carries its official default base URL and the scheme class it
/// accepts. Defaults are HTTPS/WSS; HTTP/WS is only permitted for custom bases
/// when the caller explicitly enables insecure transport and the host passes
/// the validator's local-host string check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiFamily {
    /// General PaaS v4 REST API.
    PaasV4,
    /// Coding-specific PaaS v4 REST API.
    CodingPaasV4,
    /// Agent v1 REST API.
    AgentV1,
    /// Shared LLM-application API base.
    LlmApplication,
    /// Application v2 routes, resolved against the LLM-application base.
    ApplicationV2,
    /// Application v3 routes, resolved against the LLM-application base.
    ApplicationV3,
    /// ZRAG API.
    Zrag,
    /// Usage and quota monitor API.
    Monitor,
    /// WebSocket realtime endpoint.
    Realtime,
}

impl ApiFamily {
    /// The official default base URL for this family.
    pub const fn default_base(self) -> &'static str {
        match self {
            ApiFamily::PaasV4 => "https://open.bigmodel.cn/api/paas/v4",
            ApiFamily::CodingPaasV4 => "https://open.bigmodel.cn/api/coding/paas/v4",
            ApiFamily::AgentV1 => "https://open.bigmodel.cn/api/v1",
            // ApplicationV2/V3 route paths include their version and share this
            // family base.
            ApiFamily::LlmApplication | ApiFamily::ApplicationV2 | ApiFamily::ApplicationV3 => {
                "https://open.bigmodel.cn/api/llm-application/open"
            },
            ApiFamily::Zrag => "https://open.bigmodel.cn/api/zrag",
            ApiFamily::Monitor => "https://open.bigmodel.cn/api/monitor",
            ApiFamily::Realtime => "wss://open.bigmodel.cn/api/paas/v4/realtime",
        }
    }

    /// Whether this family is a WebSocket family (scheme ws/wss).
    pub const fn is_realtime(self) -> bool {
        matches!(self, ApiFamily::Realtime)
    }

    /// The secure scheme for this family (`wss` for realtime, `https` otherwise).
    pub const fn secure_scheme(self) -> &'static str {
        if self.is_realtime() { "wss" } else { "https" }
    }

    /// The insecure scheme for this family (`ws` for realtime, `http` otherwise).
    pub const fn insecure_scheme(self) -> &'static str {
        if self.is_realtime() { "ws" } else { "http" }
    }
}

/// A validated set of per-family base URLs.
///
/// Each field is a parsed [`Url`]; the field is private and only reachable via
/// [`EndpointConfig::resolve`], which percent-encodes dynamic segments. There is
/// no string-fallback path.
#[derive(Clone)]
pub struct EndpointConfig {
    paas_v4: Url,
    coding_paas_v4: Url,
    agent_v1: Url,
    llm_application: Url,
    zrag: Url,
    monitor: Url,
    realtime: Url,
}

impl std::fmt::Debug for EndpointConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EndpointConfig")
            .field("paas_v4", &self.paas_v4.as_str())
            .field("coding_paas_v4", &self.coding_paas_v4.as_str())
            .field("agent_v1", &self.agent_v1.as_str())
            .field("llm_application", &self.llm_application.as_str())
            .field("zrag", &self.zrag.as_str())
            .field("monitor", &self.monitor.as_str())
            .field("realtime", &self.realtime.as_str())
            .finish()
    }
}

impl EndpointConfig {
    /// Build the default (official) endpoint set.
    pub fn defaults() -> ZaiResult<Self> {
        Self::builder().build(false)
    }

    /// Start a builder.
    pub fn builder() -> EndpointConfigBuilder {
        EndpointConfigBuilder {
            paas_v4: ApiFamily::PaasV4.default_base().to_string(),
            coding_paas_v4: ApiFamily::CodingPaasV4.default_base().to_string(),
            agent_v1: ApiFamily::AgentV1.default_base().to_string(),
            llm_application: ApiFamily::LlmApplication.default_base().to_string(),
            zrag: ApiFamily::Zrag.default_base().to_string(),
            monitor: ApiFamily::Monitor.default_base().to_string(),
            realtime: ApiFamily::Realtime.default_base().to_string(),
        }
    }

    /// Replace one validated family base while preserving every other base.
    /// Insecure HTTP/WS bases are accepted only for literal loopback hosts when
    /// `allow_insecure` is true.
    pub fn with_base(
        mut self,
        family: ApiFamily,
        base: impl AsRef<str>,
        allow_insecure: bool,
    ) -> ZaiResult<Self> {
        let parsed = parse_family_base(base.as_ref(), family, allow_insecure)?;
        match family {
            ApiFamily::PaasV4 => self.paas_v4 = parsed,
            ApiFamily::CodingPaasV4 => self.coding_paas_v4 = parsed,
            ApiFamily::AgentV1 => self.agent_v1 = parsed,
            ApiFamily::LlmApplication | ApiFamily::ApplicationV2 | ApiFamily::ApplicationV3 => {
                self.llm_application = parsed;
            },
            ApiFamily::Zrag => self.zrag = parsed,
            ApiFamily::Monitor => self.monitor = parsed,
            ApiFamily::Realtime => self.realtime = parsed,
        }
        Ok(self)
    }

    /// The validated base [`Url`] for `family`.
    pub fn base(&self, family: ApiFamily) -> &Url {
        match family {
            ApiFamily::PaasV4 => &self.paas_v4,
            ApiFamily::CodingPaasV4 => &self.coding_paas_v4,
            ApiFamily::AgentV1 => &self.agent_v1,
            ApiFamily::LlmApplication | ApiFamily::ApplicationV2 | ApiFamily::ApplicationV3 => {
                &self.llm_application
            },
            ApiFamily::Zrag => &self.zrag,
            ApiFamily::Monitor => &self.monitor,
            ApiFamily::Realtime => &self.realtime,
        }
    }

    /// Return whether any HTTP API family resolves to a loopback host.
    ///
    /// Realtime uses its own WebSocket transport, so it is deliberately not
    /// considered here. The HTTP client uses this signal to provision a
    /// proxy-free connection pool: a loopback URL must remain local even when
    /// the downstream process has configured a system proxy.
    pub(crate) fn has_loopback_http_base(&self) -> bool {
        [
            &self.paas_v4,
            &self.coding_paas_v4,
            &self.agent_v1,
            &self.llm_application,
            &self.zrag,
            &self.monitor,
        ]
        .into_iter()
        .any(url_is_loopback)
    }

    /// Resolve `path_segments` (applied in order) against the family base,
    /// returning a fully-formed URL string. Each segment is percent-encoded;
    /// empty, `.` and `..` segments are rejected.
    ///
    /// Pass an empty slice for the bare family base.
    pub fn resolve(&self, family: ApiFamily, segments: &[&str]) -> ZaiResult<String> {
        let mut url = self.base(family).clone();
        if segments.is_empty() {
            return Ok(url.to_string());
        }
        // Validate every segment before mutating, so a later bad segment does
        // not partially corrupt the URL.
        for seg in segments {
            validate_segment(seg)?;
        }
        let mut path = url
            .path_segments_mut()
            .map_err(|_| invalid("base URL cannot be a base"))?;
        for seg in segments {
            path.push(seg);
        }
        drop(path);
        Ok(url.to_string())
    }

    /// Resolve a family base + segments + query pairs into a URL string.
    pub fn resolve_with_query(
        &self,
        family: ApiFamily,
        segments: &[&str],
        query: &[(&str, &str)],
    ) -> ZaiResult<String> {
        let mut url = self.base(family).clone();
        if !segments.is_empty() {
            for seg in segments {
                validate_segment(seg)?;
            }
            let mut path = url
                .path_segments_mut()
                .map_err(|_| invalid("base URL cannot be a base"))?;
            for seg in segments {
                path.push(seg);
            }
        }
        if !query.is_empty() {
            let mut pairs = url.query_pairs_mut();
            for (k, v) in query {
                pairs.append_pair(k, v);
            }
        }
        Ok(url.to_string())
    }

    /// Resolve a route from the crate's canonical operation registry.
    ///
    /// `parameters` are substituted into parameter slots in declaration order
    /// and percent-encoded as individual path segments. A count mismatch is a
    /// validation error instead of silently producing a malformed URL.
    #[cfg(test)]
    pub(crate) fn resolve_route(&self, route: Route, parameters: &[&str]) -> ZaiResult<String> {
        self.resolve_route_with_query(route, parameters, &[])
    }

    /// Resolve a canonical route and append encoded query pairs.
    pub(crate) fn resolve_route_with_query(
        &self,
        route: Route,
        parameters: &[&str],
        query: &[(&str, &str)],
    ) -> ZaiResult<String> {
        for parameter in parameters {
            validate_segment(parameter)?;
        }

        let expected = route
            .segments()
            .iter()
            .filter(|segment| matches!(segment, Segment::Parameter))
            .count();
        if parameters.len() != expected {
            return Err(ZaiError::ApiError {
                code: codes::SDK_VALIDATION,
                message: format!(
                    "route {} expects {expected} path parameter(s), got {}",
                    route.operation_id(),
                    parameters.len()
                ),
            });
        }

        let mut url = self.base(route.family()).clone();
        if !route.segments().is_empty() {
            let mut parameter_index = 0;
            let mut path = url
                .path_segments_mut()
                .map_err(|_| invalid("base URL cannot be a base"))?;
            for segment in route.segments() {
                match segment {
                    Segment::Static(value) => {
                        path.push(value);
                    },
                    Segment::Parameter => {
                        path.push(parameters[parameter_index]);
                        parameter_index += 1;
                    },
                }
            }
        }
        if !query.is_empty() {
            let mut pairs = url.query_pairs_mut();
            for (key, value) in query {
                pairs.append_pair(key, value);
            }
        }
        Ok(url.to_string())
    }
}

/// Builder for [`EndpointConfig`].
pub struct EndpointConfigBuilder {
    paas_v4: String,
    coding_paas_v4: String,
    agent_v1: String,
    llm_application: String,
    zrag: String,
    monitor: String,
    realtime: String,
}

impl EndpointConfigBuilder {
    /// Override the PAAS v4 base.
    pub fn paas_v4(mut self, base: impl Into<String>) -> Self {
        self.paas_v4 = base.into();
        self
    }
    /// Override the Coding PAAS v4 base.
    pub fn coding_paas_v4(mut self, base: impl Into<String>) -> Self {
        self.coding_paas_v4 = base.into();
        self
    }
    /// Override the Agent v1 base.
    pub fn agent_v1(mut self, base: impl Into<String>) -> Self {
        self.agent_v1 = base.into();
        self
    }
    /// Override the LLM-application base (also covers ApplicationV2/V3).
    pub fn llm_application(mut self, base: impl Into<String>) -> Self {
        self.llm_application = base.into();
        self
    }
    /// Override the Zrag base.
    pub fn zrag(mut self, base: impl Into<String>) -> Self {
        self.zrag = base.into();
        self
    }
    /// Override the monitor base.
    pub fn monitor(mut self, base: impl Into<String>) -> Self {
        self.monitor = base.into();
        self
    }
    /// Override the realtime base.
    pub fn realtime(mut self, base: impl Into<String>) -> Self {
        self.realtime = base.into();
        self
    }

    /// Finalize. When `allow_insecure` is false, every base must be HTTPS/WSS.
    /// When true, HTTP/WS is accepted but only for loopback/localhost hosts.
    pub fn build(self, allow_insecure: bool) -> ZaiResult<EndpointConfig> {
        Ok(EndpointConfig {
            paas_v4: parse_family_base(&self.paas_v4, ApiFamily::PaasV4, allow_insecure)?,
            coding_paas_v4: parse_family_base(
                &self.coding_paas_v4,
                ApiFamily::CodingPaasV4,
                allow_insecure,
            )?,
            agent_v1: parse_family_base(&self.agent_v1, ApiFamily::AgentV1, allow_insecure)?,
            llm_application: parse_family_base(
                &self.llm_application,
                ApiFamily::LlmApplication,
                allow_insecure,
            )?,
            zrag: parse_family_base(&self.zrag, ApiFamily::Zrag, allow_insecure)?,
            monitor: parse_family_base(&self.monitor, ApiFamily::Monitor, allow_insecure)?,
            realtime: parse_family_base(&self.realtime, ApiFamily::Realtime, allow_insecure)?,
        })
    }
}

/// Parse and validate a family base URL string.
///
/// Validation rules:
/// - must be an absolute URL (cannot-be-a-base / relative rejected);
/// - no userinfo, no query, no fragment;
/// - scheme must be the family's secure scheme, OR the insecure scheme when
///   `allow_insecure` is true AND the host is loopback/localhost.
fn parse_family_base(raw: &str, family: ApiFamily, allow_insecure: bool) -> ZaiResult<Url> {
    // Do not echo the raw value: a malformed URL may contain userinfo or query
    // credentials, and configuration errors are commonly written to logs.
    let mut url =
        Url::parse(raw).map_err(|error| invalid(&format!("invalid base URL: {error}")))?;

    if url.cannot_be_a_base() {
        return Err(invalid("base URL must be absolute, not a relative URL"));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(invalid("base URL must not contain userinfo"));
    }
    if url.query().is_some() {
        return Err(invalid("base URL must not contain a query string"));
    }
    if url.fragment().is_some() {
        return Err(invalid("base URL must not contain a fragment"));
    }

    let scheme = url.scheme();
    let host = url
        .host()
        .ok_or_else(|| invalid("base URL must contain a host"))?;
    if scheme == family.secure_scheme() {
        // Always-allowed secure scheme.
    } else if allow_insecure && scheme == family.insecure_scheme() {
        // Insecure scheme: host must be loopback/localhost.
        if !is_loopback(host) {
            return Err(invalid(&format!(
                "insecure {scheme} transport is only allowed for loopback/localhost"
            )));
        }
    } else {
        return Err(invalid(&format!(
            "family {:?} requires scheme {} (or {} on loopback); got {scheme:?}",
            family,
            family.secure_scheme(),
            family.insecure_scheme()
        )));
    }

    // `Url::path_segments_mut().push()` preserves an existing trailing empty
    // segment, which would turn a conventional `.../v4/` base into
    // `.../v4//chat/completions`. Normalize only trailing separators; encoded
    // and interior path segments remain untouched.
    while url.path().len() > 1 && url.path().ends_with('/') {
        url.path_segments_mut()
            .map_err(|_| invalid("base URL cannot be a base"))?
            .pop_if_empty();
    }

    Ok(url)
}

/// Apply the syntactic host allow-list used for insecure transport.
///
/// This function does not resolve DNS. It accepts the exact `localhost` name
/// and IPv4/IPv6 literals for which [`std::net::IpAddr::is_loopback`] is true.
pub(crate) fn is_loopback(host: url::Host<&str>) -> bool {
    match host {
        url::Host::Domain(domain) => domain.eq_ignore_ascii_case("localhost"),
        url::Host::Ipv4(address) => address.is_loopback(),
        url::Host::Ipv6(address) => address.is_loopback(),
    }
}

/// Return whether a parsed URL names the syntactically allow-listed loopback
/// host set used by the endpoint validator.
pub(crate) fn url_is_loopback(url: &Url) -> bool {
    url.host().is_some_and(is_loopback)
}

/// Reject empty, `.` and `..` path segments.
fn validate_segment(seg: &str) -> ZaiResult<()> {
    if seg.is_empty() {
        return Err(ZaiError::ApiError {
            code: codes::SDK_VALIDATION,
            message: "path segment must not be empty".to_string(),
        });
    }
    if seg == "." || seg == ".." {
        return Err(ZaiError::ApiError {
            code: codes::SDK_VALIDATION,
            message: format!("path segment must not be `{seg}`"),
        });
    }
    Ok(())
}

fn invalid(msg: &str) -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_CONFIG,
        message: msg.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_official_secure_urls() {
        let ec = EndpointConfig::defaults().unwrap();
        assert_eq!(
            ec.base(ApiFamily::PaasV4).as_str(),
            "https://open.bigmodel.cn/api/paas/v4"
        );
        assert_eq!(ec.base(ApiFamily::Realtime).scheme(), "wss",);
    }

    #[test]
    fn resolve_percent_encodes_dynamic_segments() {
        let ec = EndpointConfig::defaults().unwrap();
        // `a/b` becomes a single percent-encoded segment `a%2Fb`.
        let url = ec.resolve(ApiFamily::PaasV4, &["a/b"]).unwrap();
        assert!(url.contains("a%2Fb"), "segment was not encoded: {url}");
        // Exactly one path segment added beyond the base.
        let base = ec.base(ApiFamily::PaasV4).as_str();
        assert!(url.starts_with(base));
    }

    #[test]
    fn resolve_rejects_empty_dot_dotdot_segments() {
        let ec = EndpointConfig::defaults().unwrap();
        assert!(ec.resolve(ApiFamily::PaasV4, &[""]).is_err());
        assert!(ec.resolve(ApiFamily::PaasV4, &["."]).is_err());
        assert!(ec.resolve(ApiFamily::PaasV4, &[".."]).is_err());
    }

    #[test]
    fn resolve_with_query_appends_pairs() {
        let ec = EndpointConfig::defaults().unwrap();
        let url = ec
            .resolve_with_query(
                ApiFamily::PaasV4,
                &["files"],
                &[("limit", "10"), ("order", "desc")],
            )
            .unwrap();
        assert!(url.contains("limit=10"));
        assert!(url.contains("order=desc"));
    }

    #[test]
    fn canonical_route_encodes_parameters_and_query() {
        let ec = EndpointConfig::defaults().unwrap();
        let url = ec
            .resolve_route_with_query(
                crate::client::routes::FILES_PARSE_RESULT,
                &["task/with/slash", "md"],
                &[("name", "a&b")],
            )
            .unwrap();
        assert_eq!(
            url,
            "https://open.bigmodel.cn/api/paas/v4/files/parser/result/task%2Fwith%2Fslash/md?name=a%26b"
        );
    }

    #[test]
    fn trailing_slash_base_does_not_create_an_empty_route_segment() {
        let endpoints = EndpointConfig::builder()
            .paas_v4("http://127.0.0.1:8080/api/paas/v4/")
            .build(true)
            .unwrap();
        let url = endpoints
            .resolve_route(crate::client::routes::CHAT_COMPLETE, &[])
            .unwrap();
        assert_eq!(url, "http://127.0.0.1:8080/api/paas/v4/chat/completions");
    }

    #[test]
    fn canonical_route_rejects_parameter_count_mismatch() {
        let ec = EndpointConfig::defaults().unwrap();
        let error = ec
            .resolve_route(crate::client::routes::FILES_GET_CONTENT, &[])
            .unwrap_err();
        assert!(error.message().contains("expects 1 path parameter"));
    }

    #[test]
    fn rejects_relative_userinfo_query_fragment() {
        // relative
        assert!(
            EndpointConfig::builder()
                .paas_v4("not/a/url")
                .build(true)
                .is_err()
        );
        // userinfo
        assert!(
            EndpointConfig::builder()
                .paas_v4("https://user:pass@open.bigmodel.cn/api/paas/v4")
                .build(false)
                .is_err()
        );
        // query
        assert!(
            EndpointConfig::builder()
                .paas_v4("https://open.bigmodel.cn/api/paas/v4?x=1")
                .build(false)
                .is_err()
        );
        // fragment
        assert!(
            EndpointConfig::builder()
                .paas_v4("https://open.bigmodel.cn/api/paas/v4#frag")
                .build(false)
                .is_err()
        );
    }

    #[test]
    fn rejects_http_public_host_without_insecure() {
        // Public HTTP host, insecure not allowed → rejected.
        assert!(
            EndpointConfig::builder()
                .paas_v4("http://open.bigmodel.cn/api/paas/v4")
                .build(false)
                .is_err()
        );
    }

    #[test]
    fn http_loopback_allowed_when_insecure_enabled() {
        let ec = EndpointConfig::builder()
            .paas_v4("http://127.0.0.1:8080/api/paas/v4")
            .build(true)
            .unwrap();
        assert_eq!(ec.base(ApiFamily::PaasV4).scheme(), "http");
        assert_eq!(ec.base(ApiFamily::PaasV4).host_str(), Some("127.0.0.1"));
    }

    #[test]
    fn http_public_host_rejected_even_when_insecure_enabled() {
        // Insecure enabled but host is not loopback → still rejected.
        assert!(
            EndpointConfig::builder()
                .paas_v4("http://open.bigmodel.cn/api/paas/v4")
                .build(true)
                .is_err()
        );
    }

    #[test]
    fn insecure_dns_name_with_127_prefix_is_rejected() {
        assert!(
            EndpointConfig::builder()
                .paas_v4("http://127.evil.example/api/paas/v4")
                .build(true)
                .is_err()
        );
    }

    #[test]
    fn replacing_one_base_preserves_the_rest() {
        let defaults = EndpointConfig::defaults().unwrap();
        let updated = defaults
            .clone()
            .with_base(
                ApiFamily::Realtime,
                "wss://example.com/custom-realtime",
                false,
            )
            .unwrap();
        assert_eq!(
            updated.base(ApiFamily::PaasV4),
            defaults.base(ApiFamily::PaasV4)
        );
        assert_eq!(
            updated.base(ApiFamily::Realtime).as_str(),
            "wss://example.com/custom-realtime"
        );
    }

    #[test]
    fn ws_loopback_allowed_for_realtime_when_insecure() {
        let ec = EndpointConfig::builder()
            .realtime("ws://localhost:9000/realtime")
            .build(true)
            .unwrap();
        assert_eq!(ec.base(ApiFamily::Realtime).scheme(), "ws");
    }
}
