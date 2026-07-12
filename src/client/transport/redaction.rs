//! Standalone helpers for sanitizing correlation identifiers before tracing.
//!
//! The buffered HTTP transport does not currently call these helpers or emit a
//! correlation-ID trace. Callers that add such tracing can use
//! [`sanitize_request_id`] to retain at most [`REQUEST_ID_MAX`] printable ASCII
//! bytes.

use crate::client::transport::limits::REQUEST_ID_MAX;

/// Sanitize a server correlation `request_id`.
///
/// Non-printable and non-ASCII characters are removed, then the result is
/// truncated to at most 128 printable ASCII bytes. An input with no retained
/// characters produces an empty string.
pub fn sanitize_request_id(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .filter(|c| c.is_ascii_graphic() || *c == ' ')
        .take(REQUEST_ID_MAX)
        .collect();
    if cleaned.is_empty() {
        // Empty signals that no usable identifier remains.
        String::new()
    } else {
        cleaned
    }
}

/// `true` if a sanitized request_id carries usable information.
pub fn has_usable_id(sanitized: &str) -> bool {
    !sanitized.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_chars_replaced() {
        let raw = "abc\x01def\nghi";
        let s = sanitize_request_id(raw);
        assert!(!s.contains('\x01'));
        assert!(!s.contains('\n'));
    }

    #[test]
    fn truncated_to_max() {
        let raw = "a".repeat(500);
        let s = sanitize_request_id(&raw);
        assert_eq!(s.len(), REQUEST_ID_MAX);
    }

    #[test]
    fn empty_when_no_printable() {
        let s = sanitize_request_id("\x01\x02");
        assert!(!has_usable_id(&s));
    }
}
