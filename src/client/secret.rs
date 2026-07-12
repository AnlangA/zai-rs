//! Secret-string wrapper for API keys.
//!
//! `ApiSecret` wraps [`secrecy::SecretString`]. Its `Debug` and `Display`
//! implementations print `[REDACTED]`, while [`ApiSecret::expose`] provides
//! explicit access to the plaintext for authentication boundaries. Redacted
//! formatting reduces accidental disclosure but cannot prevent a caller from
//! exposing or logging the returned string.

use std::fmt;

use secrecy::{ExposeSecret, SecretString};

/// A redacting wrapper around the Zhipu API key.
///
/// Construct via [`ApiSecret::new`]; never construct a bare `SecretString`
/// elsewhere in the crate. `Debug` and `Display` do not reveal the plaintext;
/// cloning produces another secret wrapper.
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

// Keep the crate-wide redaction marker stable instead of exposing secrecy's
// implementation-specific formatting.
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
