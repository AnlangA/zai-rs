//! Shared validation for MCP request wire models.

use crate::ZaiResult;

pub(super) fn validate_required(fields: &[(&'static str, &str)]) -> ZaiResult<()> {
    if let Some((name, _)) = fields.iter().find(|(_, value)| value.trim().is_empty()) {
        return Err(crate::client::validation::invalid(format!(
            "MCP request field `{name}` must not be empty"
        )));
    }
    Ok(())
}

pub(super) fn validate_optional(name: &'static str, value: Option<&str>) -> ZaiResult<()> {
    if value.is_some_and(|value| value.trim().is_empty()) {
        return Err(crate::client::validation::invalid(format!(
            "MCP request field `{name}` must not be empty when provided"
        )));
    }
    Ok(())
}
