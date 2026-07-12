//! Secret-string wrapper for the API key (plan P02.1).
//!
//! `ApiSecret` wraps [`secrecy::SecretString`]. Its `Clone`, `Debug` and
//! `Display` implementations ALWAYS print `[REDACTED]` — the plaintext is only
//! reachable through [`ApiSecret::expose`], which the SDK uses only at audited
//! authentication boundaries: building an `Authorization` header or supplying
//! credentials to an SDK-managed local MCP process. This makes accidental
//! logging of the key a compile-time-shaped impossibility rather than a
//! discipline.

use std::fmt;

use secrecy::{ExposeSecret, SecretString};

/// A redacting wrapper around the Zhipu API key.
///
/// Construct via [`ApiSecret::new`]; never construct a bare `SecretString`
/// elsewhere in the crate. `Debug`/`Display`/`Clone` are guaranteed to keep the
/// secret hidden.
#[derive(Clone)]
pub struct ApiSecret(SecretString);

impl ApiSecret {
    /// Wrap a key. The input is moved into a `SecretString` (heap-allocated,
    /// zeroized on drop).
    pub fn new(key: impl Into<String>) -> Self {
        Self(SecretString::from(key.into()))
    }

    /// Borrow the plaintext at an authentication boundary.
    ///
    /// Callers must not log it or retain additional copies. HTTP callers must
    /// mark the resulting `HeaderValue` as sensitive.
    pub fn expose(&self) -> &str {
        // secrecy exposes the inner &str via `ExposeSecret::expose_secret()`;
        // we keep its use limited to small, audited authentication call sites.
        self.0.expose_secret()
    }
}

// `SecretString` already implements these via secrecy; assert the trait bounds
// hold by routing through them. These impls are what guarantee `Clone` does not
// leak and `Debug` shows nothing.
impl fmt::Debug for ApiSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Intentionally hard-code the redaction marker rather than delegating
        // to `SecretString`'s `DebugSecret` (which prints `***`) so the marker
        // matches the rest of the SDK's redaction output.
        write!(f, "[REDACTED]")
    }
}

impl fmt::Display for ApiSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[REDACTED]")
    }
}

impl Default for ApiSecret {
    fn default() -> Self {
        Self(SecretString::from(String::new()))
    }
}

// `SecretString` (secrecy 0.10.3) is a `SecretBox<str>` with a direct `Clone`
// impl and zeroizes on drop; its plaintext is reachable only through
// `ExposeSecret::expose_secret`. `ApiSecret` keeps a single audited call site
// for that (see `expose`).

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_and_display_are_redacted() {
        let s = ApiSecret::new("abcdefghij.0123456789abcdef");
        assert_eq!(format!("{s:?}"), "[REDACTED]");
        assert_eq!(format!("{s}"), "[REDACTED]");
    }

    #[test]
    fn expose_returns_plaintext() {
        let s = ApiSecret::new("abcdefghij.0123456789abcdef");
        assert_eq!(s.expose(), "abcdefghij.0123456789abcdef");
    }

    #[test]
    fn clone_does_not_leak_and_remains_exposable() {
        let s = ApiSecret::new("abcdefghij.0123456789abcdef");
        let cloned = s.clone();
        assert_eq!(format!("{cloned:?}"), "[REDACTED]");
        assert_eq!(cloned.expose(), "abcdefghij.0123456789abcdef");
    }
}
