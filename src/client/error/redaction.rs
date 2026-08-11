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
        (
            // JSON error envelopes frequently quote both the field name and
            // value. Match a complete JSON string (including escaped bytes)
            // so commas or whitespace inside the credential cannot leave a
            // reconstructable suffix behind.
            r#"(?i)("(?:[a-z0-9_-]*(?:api[_-]?key|password|passwd|token|secret|credential(?:s)?|private[_-]?key|secret[_-]?key)|authorization)"\s*[=:]\s*)"(?:\\.|[^"\\])*""#,
            r#"$1"[FILTERED]""#,
        ),
        (
            // Some providers and loggers emit JavaScript-style object
            // fragments with single quotes instead of strict JSON.
            r#"(?i)('(?:[a-z0-9_-]*(?:api[_-]?key|password|passwd|token|secret|credential(?:s)?|private[_-]?key|secret[_-]?key)|authorization)'\s*[=:]\s*)'(?:\\.|[^'\\])*'"#,
            "$1'[FILTERED]'",
        ),
        (
            // A diagnostic prefix may end in the middle of a quoted value.
            // Treat the rest of that bounded prefix as sensitive even though
            // the provider envelope is no longer valid JSON.
            r#"(?is)("(?:[a-z0-9_-]*(?:api[_-]?key|password|passwd|token|secret|credential(?:s)?|private[_-]?key|secret[_-]?key)|authorization)"\s*[=:]\s*)"(?:\\.|[^"\\])*\\?$"#,
            r#"$1"[FILTERED]""#,
        ),
        (
            r#"(?is)('(?:[a-z0-9_-]*(?:api[_-]?key|password|passwd|token|secret|credential(?:s)?|private[_-]?key|secret[_-]?key)|authorization)'\s*[=:]\s*)'(?:\\.|[^'\\])*\\?$"#,
            "$1'[FILTERED]'",
        ),
        (
            // Fail closed for truncated/malformed envelopes whose quoted key
            // is followed by an unquoted or structured value. Once strict
            // parsing has failed there is no trustworthy value boundary, so
            // discard the rest of the diagnostic line. Excluding both quote
            // styles prevents this fallback from rewriting the complete and
            // EOF-truncated string forms above.
            r#"(?is)("(?:[a-z0-9_-]*(?:api[_-]?key|password|passwd|token|secret|credential(?:s)?|private[_-]?key|secret[_-]?key)|authorization)"\s*[=:]\s*)[^"'\r\n].*$"#,
            "$1[FILTERED]",
        ),
        (
            r#"(?is)('(?:[a-z0-9_-]*(?:api[_-]?key|password|passwd|token|secret|credential(?:s)?|private[_-]?key|secret[_-]?key)|authorization)'\s*[=:]\s*)[^"'\r\n].*$"#,
            "$1[FILTERED]",
        ),
        (r"(?i)(api[_-]?key\s*[=:]\s*)[^,\r\n]+", "$1[FILTERED]"),
        (r"(?i)(password\s*[=:]\s*)[^,\r\n]+", "$1[FILTERED]"),
        (
            r"(?i)([a-z0-9_-]*(?:api[_-]?key|password|passwd|token|secret|credential(?:s)?|private[_-]?key|secret[_-]?key)\s*[=:]\s*)[^,\r\n]+",
            "$1[FILTERED]",
        ),
        (r"(?i)(token\s*[=:]\s*)[^,\r\n]+", "$1[FILTERED]"),
        (r"(?i)(secret\s*[=:]\s*)[^,\r\n]+", "$1[FILTERED]"),
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
        r#"(?i)"(?:[a-z0-9_-]*(?:api[_-]?key|password|passwd|token|secret|credential(?:s)?|private[_-]?key|secret[_-]?key)|authorization)"\s*[=:]"#,
        r#"(?i)'(?:[a-z0-9_-]*(?:api[_-]?key|password|passwd|token|secret|credential(?:s)?|private[_-]?key|secret[_-]?key)|authorization)'\s*[=:]"#,
        r"(?i)api[_-]?key\s*[=:]",
        r"(?i)password\s*[=:]",
        r"(?i)[a-z0-9_-]*(?:api[_-]?key|password|passwd|token|secret|credential(?:s)?|private[_-]?key|secret[_-]?key)\s*[=:]",
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
/// - Quoted JSON credential fields (including `authorization`)
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
    let decoded_sensitive_key = contains_decoded_sensitive_json_key(text);
    if looks_like_json(text)
        && let Ok(Some(redacted)) = redact_structured_json(text)
    {
        return redacted;
    }
    if contains_one_more_encoded_sensitive_layer(text) {
        return "[FILTERED]".to_owned();
    }
    // If a JSON-looking fragment is malformed, decoded key recognition still
    // catches escapes such as `to\u006ben`. Without a trustworthy value
    // boundary, discard the whole diagnostic rather than risk a suffix leak.
    if decoded_sensitive_key {
        return "[FILTERED]".to_owned();
    }
    // The return type is owned, but a clean message should require only that
    // one final copy. Calling `replace_all(...).into_owned()` for every pattern
    // otherwise reallocates the full string repeatedly even when nothing
    // matches, which is the overwhelmingly common error/logging path.
    if !contains_sensitive_pattern(text) {
        return text.to_owned();
    }
    mask_unstructured_sensitive_info(text)
}

fn mask_unstructured_sensitive_info(text: &str) -> String {
    let mut result = API_KEY_PATTERN.replace_all(text, "[FILTERED]").into_owned();
    for (pattern, replacement) in SENSITIVE_PATTERNS.iter() {
        result = pattern.replace_all(&result, *replacement).into_owned();
    }
    result
}

fn redact_structured_json(text: &str) -> serde_json::Result<Option<String>> {
    let mut value: serde_json::Value = serde_json::from_str(text)?;
    if !redact_json_value(&mut value) {
        return Ok(None);
    }
    serde_json::to_string(&value).map(Some)
}

fn redact_json_value(value: &mut serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(fields) => {
            let mut changed = false;
            let original = std::mem::take(fields);
            for (key, mut value) in original {
                if is_sensitive_field(&key) {
                    value = serde_json::Value::String("[FILTERED]".to_owned());
                    changed = true;
                } else {
                    changed |= redact_json_value(&mut value);
                }
                let redacted_key = if contains_one_more_encoded_sensitive_layer(&key) {
                    "[FILTERED]".to_owned()
                } else {
                    mask_unstructured_sensitive_info(&key)
                };
                changed |= redacted_key != key;
                fields.insert(redacted_key, value);
            }
            changed
        },
        serde_json::Value::Array(values) => {
            let mut changed = false;
            for value in values {
                changed |= redact_json_value(value);
            }
            changed
        },
        serde_json::Value::String(text) => {
            if contains_decoded_sensitive_json_key(text)
                || contains_one_more_encoded_sensitive_layer(text)
            {
                *text = "[FILTERED]".to_owned();
                return true;
            }
            let redacted = mask_unstructured_sensitive_info(text);
            if redacted == *text {
                false
            } else {
                *text = redacted;
                true
            }
        },
        _ => false,
    }
}

fn is_sensitive_field(field: &str) -> bool {
    let normalized: String = field
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|character| character.to_ascii_lowercase())
        .collect();
    normalized == "authorization"
        || normalized.ends_with("apikey")
        || normalized.ends_with("password")
        || normalized.ends_with("passwd")
        || normalized.ends_with("token")
        || normalized.ends_with("secret")
        || normalized.ends_with("secretkey")
        || normalized.ends_with("privatekey")
        || normalized.ends_with("credential")
        || normalized.ends_with("credentials")
}

fn looks_like_json(text: &str) -> bool {
    matches!(text.trim_start().as_bytes().first(), Some(b'{' | b'['))
}

fn contains_decoded_sensitive_json_key(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut cursor = 0;
    while let Some(offset) = bytes[cursor..].iter().position(|byte| *byte == b'"') {
        let start = cursor + offset;
        let Some(end) = json_string_end(bytes, start) else {
            return false;
        };
        let mut after = end;
        while bytes
            .get(after)
            .is_some_and(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
        {
            after += 1;
        }
        if bytes.get(after) == Some(&b':')
            && serde_json::from_str::<String>(&text[start..end])
                .ok()
                .is_some_and(|field| is_sensitive_field(&field))
        {
            return true;
        }
        cursor = end;
    }
    false
}

fn json_string_end(input: &[u8], start: usize) -> Option<usize> {
    if input.get(start) != Some(&b'"') {
        return None;
    }
    let mut cursor = start + 1;
    let mut escaped = false;
    while cursor < input.len() {
        match (input[cursor], escaped) {
            (_, true) => escaped = false,
            (b'\\', false) => escaped = true,
            (b'"', false) => return Some(cursor + 1),
            _ => {},
        }
        cursor += 1;
    }
    None
}

fn contains_one_more_encoded_sensitive_layer(text: &str) -> bool {
    if !text.contains('\\') {
        return false;
    }
    let mut encoded = String::with_capacity(text.len() + 2);
    encoded.push('"');
    encoded.push_str(text);
    encoded.push('"');
    let Ok(decoded) = serde_json::from_str::<String>(&encoded) else {
        return false;
    };
    decoded != text
        && (contains_sensitive_pattern(&decoded) || contains_decoded_sensitive_json_key(&decoded))
}

/// Mask API keys that follow the Zhipu AI `id.secret` format.
pub fn mask_api_key(text: &str) -> String {
    API_KEY_PATTERN.replace_all(text, "[FILTERED]").into_owned()
}

/// Return whether `text` matches a recognized credential pattern.
pub fn contains_sensitive_info(text: &str) -> bool {
    (looks_like_json(text) && structured_json_contains_sensitive(text))
        || contains_one_more_encoded_sensitive_layer(text)
        || contains_sensitive_pattern(text)
        || contains_decoded_sensitive_json_key(text)
}

fn structured_json_contains_sensitive(text: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .is_some_and(|mut value| redact_json_value(&mut value))
}

fn contains_sensitive_pattern(text: &str) -> bool {
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
