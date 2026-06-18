//! JWT generation for GLM-Realtime client-side authentication.
//!
//! The realtime WebSocket supports two auth modes (see
//! <https://github.com/MetaGLM/glm-realtime-sdk/blob/main/GLM-Realtime-doc-for-llm.md>):
//!
//! - **Bearer** (server-side): `Authorization: Bearer {API_KEY}`.
//! - **JWT** (client-side): a JWT signed with the API key's secret, so the real
//!   API key never reaches the browser/device. This module implements that JWT.
//!
//! The GLM JWT is **non-standard**: header
//! `{"alg":"HS256","sign_type":"SIGN""}` and payload
//! `{"api_key":id,"exp":secs,"timestamp":ms}`, signed with HMAC-SHA256 keyed by
//! the secret half of the `{id}.{secret}` API key.

use base64::Engine;
use chrono::Utc;
use hmac::{Hmac, Mac};
use serde_json::json;
use sha2::Sha256;
use tracing::warn;

use crate::{ZaiResult, client::error::ZaiError};

type HmacSha256 = Hmac<Sha256>;

/// Build the `Authorization` header value for the realtime handshake.
///
/// Always returns a `Bearer <token>` string. With `jwt_seconds = Some(ttl)` the
/// token is a freshly-signed JWT derived from the API key; with `None` it is
/// the raw API key (server-side Bearer auth).
pub fn authorization_header(api_key: &str, jwt_seconds: Option<i64>) -> ZaiResult<String> {
    match jwt_seconds {
        Some(ttl) => Ok(format!("Bearer {}", generate(api_key, ttl)?)),
        None => Ok(format!("Bearer {api_key}")),
    }
}

/// Sign a GLM-Realtime JWT for the given `{id}.{secret}` API key.
pub fn generate(api_key: &str, ttl_seconds: i64) -> ZaiResult<String> {
    let (id, secret) = match api_key.split_once('.') {
        Some(parts) => parts,
        None => {
            warn!("Realtime JWT auth error: API key must be '<id>.<secret>'");
            return Err(ZaiError::RealtimeAuthError(
                "API key must be '<id>.<secret>'".into(),
            ));
        }
    };

    if id.is_empty() || secret.is_empty() {
        warn!("Realtime JWT auth error: API key id and secret must be non-empty");
        return Err(ZaiError::RealtimeAuthError(
            "API key id and secret must be non-empty".into(),
        ));
    }

    let now = Utc::now();
    let exp = now.timestamp() + ttl_seconds;
    let timestamp_ms = now.timestamp_millis();

    let header = json!({ "alg": "HS256", "sign_type": "SIGN" });
    let payload = json!({
        "api_key": id,
        "exp": exp,
        "timestamp": timestamp_ms,
    });

    let header_b64 = base64url(serde_json::to_string(&header)?.as_bytes());
    let payload_b64 = base64url(serde_json::to_string(&payload)?.as_bytes());
    let signing_input = format!("{header_b64}.{payload_b64}");

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).map_err(|e| {
        warn!("Realtime JWT auth error: HMAC key error");
        ZaiError::RealtimeAuthError(format!("HMAC key error: {e}"))
    })?;
    mac.update(signing_input.as_bytes());
    let sig = mac.finalize().into_bytes();
    let sig_b64 = base64url(sig.as_slice());

    Ok(format!("{signing_input}.{sig_b64}"))
}

fn base64url(data: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}

#[cfg(test)]
mod tests {
    use base64::Engine;

    use super::*;

    #[test]
    fn jwt_has_three_urlsafe_segments() {
        let token = generate("abcdefghij.0123456789abcdef", 600).unwrap();
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3, "JWT must have header.payload.signature");
        // URL-safe, no padding.
        assert!(!token.contains('='));
        assert!(!token.contains('+'));
        assert!(!token.contains('/'));
    }

    #[test]
    fn jwt_header_and_payload_decode_to_expected_fields() {
        let token = generate("abcdefghij.0123456789abcdef", 600).unwrap();
        let mut parts = token.split('.');
        let header_json = String::from_utf8(
            base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(parts.next().unwrap())
                .unwrap(),
        )
        .unwrap();
        let payload_json = String::from_utf8(
            base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(parts.next().unwrap())
                .unwrap(),
        )
        .unwrap();
        assert!(header_json.contains("\"alg\":\"HS256\""));
        assert!(header_json.contains("\"sign_type\":\"SIGN\""));
        assert!(payload_json.contains("\"api_key\":\"abcdefghij\""));
        assert!(payload_json.contains("\"exp\""));
        assert!(payload_json.contains("\"timestamp\""));
    }

    #[test]
    fn rejects_malformed_keys() {
        assert!(generate("no-dot-here", 600).is_err());
        assert!(generate(".secret", 600).is_err());
        assert!(generate("id.", 600).is_err());
    }

    #[test]
    fn authorization_header_bearer_jwt_and_raw() {
        let h = authorization_header("abcdefghij.0123456789abcdef", Some(3600)).unwrap();
        assert!(h.starts_with("Bearer "));
        assert!(h[7..].split('.').count() == 3);

        let h2 = authorization_header("abcdefghij.0123456789abcdef", None).unwrap();
        assert_eq!(h2, "Bearer abcdefghij.0123456789abcdef");
    }
}
