//! JWT generation for GLM-Realtime authentication.
//!
//! The realtime WebSocket supports two auth modes (see
//! <https://github.com/MetaGLM/glm-realtime-sdk/blob/main/GLM-Realtime-doc-for-llm.md>):
//!
//! - **Bearer** (server-side): `Authorization: Bearer {API_KEY}`.
//! - **JWT**: a short-lived JWT signed locally with the API key's secret. Only
//!   the derived token is sent in the WebSocket handshake. Applications that
//!   use an untrusted browser/device must generate the token on a trusted server.
//!
//! The GLM JWT is **non-standard**: header
//! `{"alg":"HS256","sign_type":"SIGN"}` and payload
//! `{"api_key":id,"exp":secs,"timestamp":ms}`, signed with HMAC-SHA256 keyed by
//! the secret half of the `{id}.{secret}` API key.

use base64::Engine;
use chrono::Utc;
use hmac::{Hmac, KeyInit, Mac};
use serde_json::json;
use sha2::Sha256;

use crate::{ZaiResult, client::error::ZaiError};

type HmacSha256 = Hmac<Sha256>;
const MAX_TTL_SECONDS: i64 = 7 * 24 * 60 * 60;

/// Build the `Authorization` header value for the realtime handshake.
///
/// Always returns a `Bearer <token>` string. With `jwt_seconds = Some(ttl)` the
/// token is a freshly-signed JWT derived from the API key; with `None` it is
/// the raw API key (server-side Bearer auth).
pub fn authorization_header(api_key: &str, jwt_seconds: Option<i64>) -> ZaiResult<String> {
    crate::client::error::validate_api_key(api_key).map_err(|error| {
        ZaiError::RealtimeAuthError(format!("invalid API key: {}", error.message()))
    })?;
    match jwt_seconds {
        Some(ttl) => Ok(format!("Bearer {}", generate(api_key, ttl)?)),
        None => Ok(format!("Bearer {api_key}")),
    }
}

/// Sign a GLM-Realtime JWT for the given `{id}.{secret}` API key.
///
/// `ttl_seconds` must be in `1..=604_800` (seven days).
pub fn generate(api_key: &str, ttl_seconds: i64) -> ZaiResult<String> {
    crate::client::error::validate_api_key(api_key).map_err(|error| {
        ZaiError::RealtimeAuthError(format!("invalid API key: {}", error.message()))
    })?;
    let (id, secret) = api_key.split_once('.').ok_or_else(|| {
        ZaiError::RealtimeAuthError("API key must be '<id>.<secret>'".to_string())
    })?;

    // Validate ttl: a non-positive or absurdly large value would otherwise wrap
    // `exp` silently (unguarded `i64` add, inconsistent with the crate's
    // saturating-math pass) and surface as an opaque auth rejection. Seven days
    // is a generous ceiling for any realistic JWT lifetime.
    if ttl_seconds <= 0 || ttl_seconds > MAX_TTL_SECONDS {
        return Err(ZaiError::RealtimeAuthError(format!(
            "jwt ttl_seconds must be in 1..={MAX_TTL_SECONDS}, got {ttl_seconds}"
        )));
    }

    let now = Utc::now();
    // Defense-in-depth: `now + ttl` can't overflow once ttl is bounded above, but
    // use `checked_add` so a future change to the bound can't reintroduce a
    // silent wrap.
    let exp = now
        .timestamp()
        .checked_add(ttl_seconds)
        .ok_or_else(|| ZaiError::RealtimeAuthError("jwt exp overflow".into()))?;
    let timestamp_ms = now.timestamp_millis();

    let header = json!({ "alg": "HS256", "sign_type": "SIGN" });
    let payload = json!({
        "api_key": id,
        "exp": exp,
        "timestamp": timestamp_ms,
    });

    let header_b64 = base64url(&serde_json::to_vec(&header)?);
    let payload_b64 = base64url(&serde_json::to_vec(&payload)?);
    let signing_input = format!("{header_b64}.{payload_b64}");

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| ZaiError::RealtimeAuthError(format!("HMAC key error: {e}")))?;
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
        assert!(generate("id.secret.extra", 600).is_err());
    }

    #[test]
    fn rejects_out_of_range_ttl() {
        // Non-positive and absurdly large values are rejected up front rather
        // than producing a nonsense/overflowing `exp`.
        assert!(generate("abcdefghij.0123456789abcdef", 0).is_err());
        assert!(generate("abcdefghij.0123456789abcdef", -1).is_err());
        assert!(generate("abcdefghij.0123456789abcdef", i64::MAX).is_err());
        // The 7-day ceiling is inclusive; one second over is rejected.
        assert!(generate("abcdefghij.0123456789abcdef", 7 * 24 * 3600).is_ok());
        assert!(generate("abcdefghij.0123456789abcdef", 7 * 24 * 3600 + 1).is_err());
    }

    #[test]
    fn authorization_header_bearer_jwt_and_raw() {
        let h = authorization_header("abcdefghij.0123456789abcdef", Some(3600)).unwrap();
        assert!(h.starts_with("Bearer "));
        assert!(h[7..].split('.').count() == 3);

        let h2 = authorization_header("abcdefghij.0123456789abcdef", None).unwrap();
        assert_eq!(h2, "Bearer abcdefghij.0123456789abcdef");
        assert!(authorization_header("  ", None).is_err());
        assert!(authorization_header("abc\ndef", None).is_err());
    }
}
