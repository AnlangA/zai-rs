//! Shared validation helpers for request identifiers.

use crate::{ZaiError, ZaiResult, client::error::codes};

/// Construct the standard error used for locally rejected request parameters.
pub(crate) fn invalid(message: impl Into<String>) -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_VALIDATION,
        message: message.into(),
    }
}

/// Reject a required identifier containing only whitespace.
pub(crate) fn require_non_blank(value: &str, name: &'static str) -> ZaiResult<()> {
    if value.trim().is_empty() {
        Err(invalid(format!("{name} must not be blank")))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_identifier_rejects_empty_and_whitespace() {
        assert!(require_non_blank("id-1", "resource_id").is_ok());
        assert!(require_non_blank("", "resource_id").is_err());
        assert!(require_non_blank(" \t", "resource_id").is_err());
    }
}
