//! Environment-backed startup configuration.

use std::{env, net::IpAddr, str::FromStr};

use axum::http::{HeaderValue, Uri};

/// Validated configuration used to construct the server.
pub struct Config {
    pub bind_address: IpAddr,
    pub port: u16,
    pub api_key: String,
    pub cors_origins: Vec<HeaderValue>,
    pub session_timeout_secs: u64,
    pub max_messages_per_session: usize,
}

impl Config {
    /// Read configuration from environment variables and reject invalid or
    /// unsafe values before the listener starts.
    pub fn from_env() -> Result<Self, ConfigError> {
        let bind_address = parse_env("BIND_ADDRESS", "127.0.0.1")?;
        let port = parse_env("PORT", "3000")?;
        let session_timeout_secs = parse_env("SESSION_TIMEOUT", "3600")?;
        let max_messages_per_session = parse_env("MAX_MESSAGES_PER_SESSION", "1000")?;

        let api_key = env::var("ZHIPU_API_KEY").map_err(|_| ConfigError::MissingApiKey)?;
        if api_key.trim().is_empty() {
            return Err(ConfigError::MissingApiKey);
        }
        if !api_key.bytes().all(|byte| byte.is_ascii_graphic()) {
            return Err(ConfigError::InvalidApiKey);
        }
        if port == 0 {
            return Err(ConfigError::OutOfRange("PORT"));
        }
        if session_timeout_secs == 0 {
            return Err(ConfigError::OutOfRange("SESSION_TIMEOUT"));
        }
        if max_messages_per_session == 0 {
            return Err(ConfigError::OutOfRange("MAX_MESSAGES_PER_SESSION"));
        }

        let default_origins = format!("http://localhost:{port},http://127.0.0.1:{port}");
        let origins = env::var("CORS_ORIGINS").unwrap_or(default_origins);
        let cors_origins = origins
            .split(',')
            .map(str::trim)
            .filter(|origin| !origin.is_empty())
            .map(parse_cors_origin)
            .collect::<Result<Vec<_>, _>>()?;
        if cors_origins.is_empty() {
            return Err(ConfigError::InvalidCorsOrigin);
        }

        Ok(Self {
            bind_address,
            port,
            api_key,
            cors_origins,
            session_timeout_secs,
            max_messages_per_session,
        })
    }
}

fn parse_cors_origin(origin: &str) -> Result<HeaderValue, ConfigError> {
    let uri = Uri::from_str(origin).map_err(|_| ConfigError::InvalidCorsOrigin)?;
    let valid_scheme = matches!(uri.scheme_str(), Some("http" | "https"));
    let valid_authority = uri
        .authority()
        .is_some_and(|authority| !authority.as_str().contains('@'));
    let valid_target = uri.path() == "/" && uri.query().is_none() && !origin.ends_with('/');
    if !valid_scheme || !valid_authority || !valid_target {
        return Err(ConfigError::InvalidCorsOrigin);
    }
    HeaderValue::from_str(origin).map_err(|_| ConfigError::InvalidCorsOrigin)
}

fn parse_env<T>(name: &'static str, default: &str) -> Result<T, ConfigError>
where
    T: FromStr,
{
    let value = env::var(name).unwrap_or_else(|_| default.to_owned());
    value
        .parse()
        .map_err(|_| ConfigError::InvalidValue { name, value })
}

/// Configuration errors reported before any network listener is opened.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("ZHIPU_API_KEY is missing or empty")]
    MissingApiKey,
    #[error("ZHIPU_API_KEY must contain printable ASCII without whitespace")]
    InvalidApiKey,
    #[error("{name} has an invalid value: {value}")]
    InvalidValue { name: &'static str, value: String },
    #[error("{0} must be greater than zero")]
    OutOfRange(&'static str),
    #[error("CORS_ORIGINS contains an invalid HTTP(S) origin")]
    InvalidCorsOrigin,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cors_origins_require_http_origin_syntax() {
        assert!(parse_cors_origin("https://example.com").is_ok());
        assert!(parse_cors_origin("http://127.0.0.1:3000").is_ok());

        for invalid in [
            "*",
            "example.com",
            "ftp://example.com",
            "https://user@example.com",
            "https://example.com/",
            "https://example.com/path",
            "https://example.com?query=1",
        ] {
            assert!(parse_cors_origin(invalid).is_err(), "accepted {invalid}");
        }
    }

    #[test]
    fn invalid_cors_errors_never_echo_rejected_credentials() {
        let rejected = "https://user:password@example.com";
        let error = parse_cors_origin(rejected).unwrap_err();
        let display = error.to_string();
        let debug = format!("{error:?}");

        for sensitive in [rejected, "user", "password"] {
            assert!(!display.contains(sensitive));
            assert!(!debug.contains(sensitive));
        }
        assert_eq!(display, "CORS_ORIGINS contains an invalid HTTP(S) origin");
    }
}
