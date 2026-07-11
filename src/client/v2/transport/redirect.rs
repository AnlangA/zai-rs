//! Redirect policy (plan P03.11).
//!
//! The SDK's reqwest client is built with `redirect::Policy::none()`, so the
//! Transport observes every 3xx and decides per-response whether to follow.
//!
//! Rules:
//! - NonIdempotent (no override): NEVER follow any 3xx (a body may already have
//!   been consumed server-side).
//! - GET/HEAD: follow 301/302/303/307/308.
//! - Other Idempotent (PUT/DELETE/OPTIONS): follow 307/308 only.
//! - No 301/302/303 method rewrite for any method.
//! - Max 3 hops, same-origin only, no TLS downgrade. Cross-origin → Protocol
//!   error; auth header not sent to the new origin.
//! - The raw `Location` never enters an error or trace.

use url::Url;

use crate::client::v2::transport::retry::RetrySafety;
use crate::{ZaiError, ZaiResult, client::error::codes};

/// Maximum redirect hops counted against the overall deadline (plan §4).
pub const MAX_REDIRECTS: u8 = 3;

/// Decide whether to follow a redirect, returning the resolved target URL.
///
/// `current` is the URL that produced the 3xx; `status` the HTTP status;
/// `location` the raw Location header value; `safety` the effective retry safety
/// of the original request; `method` the original method.
pub fn follow(
    current: &Url,
    status: u16,
    location: &str,
    safety: RetrySafety,
    method: &str,
    hops: u8,
) -> ZaiResult<Option<Url>> {
    if hops >= MAX_REDIRECTS {
        return Err(redirect_err("redirect limit (3) exceeded"));
    }
    // NonIdempotent never follows.
    if safety == RetrySafety::NonIdempotent {
        return Ok(None);
    }
    if !is_redirect_status(status) {
        return Ok(None);
    }
    // Method matrix.
    let allowed = match method {
        "GET" | "HEAD" => matches!(status, 301 | 302 | 303 | 307 | 308),
        // Other Idempotent methods follow 307/308 only (no method rewrite).
        _ => matches!(status, 307 | 308),
    };
    if !allowed {
        return Ok(None);
    }

    let target = current
        .join(location)
        .map_err(|e| redirect_err(&format!("invalid Location: {e}")))?;

    // Reject userinfo / fragment.
    if !target.username().is_empty() || target.password().is_some() {
        return Err(redirect_err("redirect target must not contain userinfo"));
    }
    if target.fragment().is_some() {
        return Err(redirect_err("redirect target must not contain a fragment"));
    }
    // Same-origin + no TLS downgrade: scheme must match, host must match.
    if target.scheme() != current.scheme() {
        // TLS downgrade specifically: https→http or wss→ws.
        let downgraded = matches!(
            (current.scheme(), target.scheme()),
            ("https", "http") | ("wss", "ws")
        );
        if downgraded {
            return Err(redirect_err("TLS downgrade refused"));
        }
        // Any other scheme change is cross-origin.
        return Err(redirect_err("cross-origin redirect refused"));
    }
    if target.host_str() != current.host_str() || target.port() != current.port() {
        return Err(redirect_err("cross-origin redirect refused"));
    }
    Ok(Some(target))
}

fn is_redirect_status(status: u16) -> bool {
    matches!(status, 301 | 302 | 303 | 307 | 308)
}

fn redirect_err(msg: &str) -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_VALIDATION,
        message: format!("redirect: {msg}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    #[test]
    fn nonidempotent_never_follows() {
        let cur = url("https://open.bigmodel.cn/api/paas/v4/x");
        let r = follow(
            &cur,
            302,
            "/api/paas/v4/y",
            RetrySafety::NonIdempotent,
            "POST",
            0,
        )
        .unwrap();
        assert!(r.is_none());
    }

    #[test]
    fn get_follows_301_to_same_origin() {
        let cur = url("https://open.bigmodel.cn/a");
        let r = follow(&cur, 301, "/b", RetrySafety::Idempotent, "GET", 0).unwrap();
        assert_eq!(r.unwrap().as_str(), "https://open.bigmodel.cn/b");
    }

    #[test]
    fn put_only_follows_307_308() {
        let cur = url("https://open.bigmodel.cn/a");
        // 302 not followed by PUT.
        assert!(
            follow(&cur, 302, "/b", RetrySafety::Idempotent, "PUT", 0)
                .unwrap()
                .is_none()
        );
        // 307 followed.
        let r = follow(&cur, 307, "/b", RetrySafety::Idempotent, "PUT", 0).unwrap();
        assert_eq!(r.unwrap().as_str(), "https://open.bigmodel.cn/b");
    }

    #[test]
    fn cross_origin_refused() {
        let cur = url("https://open.bigmodel.cn/a");
        let err = follow(
            &cur,
            302,
            "https://evil.example.com/b",
            RetrySafety::Idempotent,
            "GET",
            0,
        )
        .unwrap_err();
        assert!(format!("{err}").contains("cross-origin"));
    }

    #[test]
    fn tls_downgrade_refused() {
        let cur = url("https://open.bigmodel.cn/a");
        let err = follow(
            &cur,
            302,
            "http://open.bigmodel.cn/b",
            RetrySafety::Idempotent,
            "GET",
            0,
        )
        .unwrap_err();
        assert!(
            format!("{err}").contains("TLS downgrade") || format!("{err}").contains("cross-origin")
        );
    }

    #[test]
    fn redirect_limit_enforced() {
        let cur = url("https://open.bigmodel.cn/a");
        assert!(
            follow(
                &cur,
                302,
                "/b",
                RetrySafety::Idempotent,
                "GET",
                MAX_REDIRECTS
            )
            .is_err()
        );
    }
}
