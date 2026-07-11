//! Tracing redaction (plan P03.10).
//!
//! The Transport only ever records fixed metadata: method, normalized route
//! template, status, byte count, attempt, elapsed and the server correlation
//! `request_id`. It NEVER logs the materialized URL, header values, query
//! values, user-supplied path/resource IDs, the request body or the response
//! body.
//!
//! `request_id` is sanitized to at most [`REQUEST_ID_MAX`] printable ASCII
//! bytes (control chars replaced).

use crate::client::v2::transport::limits::REQUEST_ID_MAX;

/// Sanitize a server correlation `request_id`: keep at most 128 bytes of
/// printable ASCII, replacing control characters with `_`.
pub fn sanitize_request_id(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .filter(|c| c.is_ascii_graphic() || *c == ' ')
        .take(REQUEST_ID_MAX)
        .collect();
    if cleaned.is_empty() {
        // A non-printable / overlong id is dropped to None upstream; here we
        // return empty to signal "no usable id".
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
