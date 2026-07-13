//! Credential validation and defensive text redaction.

use std::sync::LazyLock;

use regex::Regex;

use super::{ZaiError, ZaiResult, codes};

/// Compile a built-in redaction pattern. Invalid literals are programmer
/// errors; failing closed is safer than silently disabling filtering.
fn built_in_regex(pattern: &str) -> Regex {
    Regex::new(pattern)
        .unwrap_or_else(|error| panic!("invalid built-in regex {pattern:?}: {error}"))
}

static API_KEY_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| built_in_regex(r"[a-zA-Z0-9_-]{3,}\.[a-zA-Z0-9_-]{10,}"));

static SENSITIVE_PATTERNS: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    [
        (r"(?i)(api[_-]?key\s*[=:]\s*)[^\s,]+", "$1[FILTERED]"),
        (r"(?i)(password\s*[=:]\s*)[^\s,]+", "$1[FILTERED]"),
        (r"(?i)(token\s*[=:]\s*)[^\s,]+", "$1[FILTERED]"),
        (r"(?i)(secret\s*[=:]\s*)[^\s,]+", "$1[FILTERED]"),
        (r"(?i)(\bbearer\s+)[^\s,]+", "$1[FILTERED]"),
        (
            // Replace the complete header so neither its scheme nor token can
            // be reconstructed from adjacent output.
            r"(?i)authorization\s*:\s*Bearer\s+[^\s,]+",
            "[AUTH_REDACTED]",
        ),
    ]
    .into_iter()
    .map(|(pattern, replacement)| (built_in_regex(pattern), replacement))
    .collect()
});

static CONTAINS_SENSITIVE_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        r"(?i)api[_-]?key\s*[=:]",
        r"(?i)password\s*[=:]",
        r"(?i)token\s*[=:]",
        r"(?i)secret\s*[=:]",
        r"(?i)\bbearer\s+[^\s,]+",
        r"(?i)authorization\s*:\s*Bearer",
    ]
    .into_iter()
    .map(built_in_regex)
    .collect()
});

/// Mask recognized credentials in `text` before it is written to a log.
///
/// Values are replaced with `[FILTERED]`; a complete Authorization header is
/// replaced with `[AUTH_REDACTED]`.
///
/// # Patterns Masked
///
/// - API keys (`id.secret`, with an id of at least 3 characters and a secret
///   of at least 10 characters)
/// - Password, token, and secret fields
/// - Bearer tokens and Authorization headers
///
/// # Example
///
/// ```
/// use zai_rs::client::error::mask_sensitive_info;
///
/// let text = "API key: abc123.abcdefghijklmnopqrstuvwxyz, password: secret123";
/// let filtered = mask_sensitive_info(text);
/// assert!(filtered.contains("[FILTERED]"));
/// assert!(!filtered.contains("abc123"));
/// ```
pub fn mask_sensitive_info(text: &str) -> String {
    let mut result = API_KEY_PATTERN.replace_all(text, "[FILTERED]").into_owned();

    for (pattern, replacement) in SENSITIVE_PATTERNS.iter() {
        result = pattern.replace_all(&result, *replacement).into_owned();
    }

    result
}

/// Mask API keys that follow the Zhipu AI `id.secret` format.
pub fn mask_api_key(text: &str) -> String {
    API_KEY_PATTERN.replace_all(text, "[FILTERED]").into_owned()
}

/// Return whether `text` matches a recognized credential pattern.
pub fn contains_sensitive_info(text: &str) -> bool {
    API_KEY_PATTERN.is_match(text)
        || CONTAINS_SENSITIVE_PATTERNS
            .iter()
            .any(|pattern| pattern.is_match(text))
}

/// Validate the Zhipu AI API key format.
///
/// Keys must contain exactly one dot separating an identifier of at least
/// three characters from a secret of at least ten characters. Both halves may
/// contain only ASCII letters, digits, `_`, or `-`.
///
/// # Errors
///
/// Returns [`ZaiError::ApiError`] with [`codes::SDK_VALIDATION`] when the key
/// does not satisfy the closed credential grammar.
pub fn validate_api_key(api_key: &str) -> ZaiResult<()> {
    if api_key.is_empty() {
        return Err(ZaiError::ApiError {
            code: codes::SDK_VALIDATION,
            message: "API key cannot be empty".to_string(),
        });
    }

    let Some((id, secret)) = api_key.split_once('.') else {
        return Err(ZaiError::ApiError {
            code: codes::SDK_VALIDATION,
            message: "API key must be in format '<id>.<secret>'".to_string(),
        });
    };

    if secret.contains('.') {
        return Err(ZaiError::ApiError {
            code: codes::SDK_VALIDATION,
            message: "API key must contain exactly one dot".to_string(),
        });
    }

    if id.is_empty() || secret.is_empty() {
        return Err(ZaiError::ApiError {
            code: codes::SDK_VALIDATION,
            message: "API key id and secret must not be empty".to_string(),
        });
    }

    let valid_chars = |part: &str| {
        part.bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    };

    if !valid_chars(id) || !valid_chars(secret) {
        return Err(ZaiError::ApiError {
            code: codes::SDK_VALIDATION,
            message: "API key contains invalid characters".to_string(),
        });
    }

    if id.len() < 3 {
        return Err(ZaiError::ApiError {
            code: codes::SDK_VALIDATION,
            message: "API key id is too short".to_string(),
        });
    }

    if secret.len() < 10 {
        return Err(ZaiError::ApiError {
            code: codes::SDK_VALIDATION,
            message: "API key secret is too short".to_string(),
        });
    }

    Ok(())
}
