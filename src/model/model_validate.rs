//! # JSON Schema Validation Module
//!
//! Provides custom validation functions for JSON Schema validation in the ZAI-RS model API.
//! This module ensures that JSON schemas used in function definitions and tool configurations
//! are valid and conform to the JSON Schema specification.
//!
//! ## Validation Functions
//!
//! - [`validate_json_schema`] - Validates JSON Schema from string input
//! - [`validate_json_schema_value`] - Validates JSON Schema from parsed JSON value
//!
//! ## Error Handling
//!
//! Both validation functions return `ValidationError` with specific error codes:
//! - `"invalid_json"` - Input string is not valid JSON
//! - `"invalid_json_schema"` - JSON is valid but not a valid JSON Schema
//!
//! ## Usage Examples
//!
//! ```rust,ignore
//! use zai_rs::model::model_validate::*;
//! use validator::Validate;
//!
//! // Validate from string
//! let schema_str = r#"{"type": "object", "properties": {"name": {"type": "string"}}}"#;
//! assert!(validate_json_schema(schema_str).is_ok());
//!
//! // Validate from parsed JSON
//! let json_value = serde_json::json!({
//!     "type": "object",
//!     "properties": {
//!         "name": {"type": "string"}
//!     }
//! });
//! assert!(validate_json_schema_value(&json_value).is_ok());
//! ```
//!
//! ## JSON Schema Requirements
//!
//! Valid JSON Schemas must:
//! - Be valid JSON syntax
//! - Conform to JSON Schema meta-schema validation
//! - Have appropriate schema structure for function parameters
//!
//! ## Integration with Validation
//!
//! These functions are designed to work with the `validator` crate's custom validation
//! system, allowing them to be used as field validators in struct definitions.

use validator::ValidationError;

/// Validates a JSON Schema from a string input.
///
/// This function parses the input string as JSON and then validates that it conforms
/// to the JSON Schema specification using the `jsonschema` crate's meta-validation.
///
/// # Arguments
///
/// * `parameters` - A string containing JSON that should represent a valid JSON Schema
///
/// # Returns
///
/// * `Ok(())` - If the input is valid JSON and a valid JSON Schema
/// * `Err(ValidationError)` - If the input is invalid JSON or not a valid JSON Schema
///
/// # Error Codes
///
/// * `"invalid_json"` - The input string is not valid JSON
/// * `"invalid_json_schema"` - The JSON is valid but not a valid JSON Schema
///
/// # Examples
///
/// ```rust,ignore
/// // Valid JSON Schema
/// let valid_schema = r#"{"type": "object", "properties": {"name": {"type": "string"}}}"#;
/// assert!(validate_json_schema(valid_schema).is_ok());
///
/// // Invalid JSON
/// let invalid_json = "{ invalid json }";
/// assert!(validate_json_schema(invalid_json).is_err());
///
/// // Invalid JSON Schema
/// let invalid_schema = r#"{"type": "invalid_type"}"#;
/// assert!(validate_json_schema(invalid_schema).is_err());
/// ```
pub fn validate_json_schema(parameters: &str) -> Result<(), ValidationError> {
    let schema_json: serde_json::Value =
        serde_json::from_str(parameters).map_err(|_| ValidationError::new("invalid_json"))?;
    if !jsonschema::meta::is_valid(&schema_json) {
        return Err(ValidationError::new("invalid_json_schema"));
    }
    Ok(())
}

/// Validates a JSON Schema from a parsed JSON value.
///
/// This function validates that the provided JSON value conforms to the JSON Schema
/// specification using the `jsonschema` crate's meta-validation.
///
/// # Arguments
///
/// * `parameters` - A reference to a `serde_json::Value` that should represent a valid JSON Schema
///
/// # Returns
///
/// * `Ok(())` - If the value is a valid JSON Schema
/// * `Err(ValidationError)` - If the value is not a valid JSON Schema
///
/// # Error Codes
///
/// * `"invalid_json_schema"` - The JSON value is not a valid JSON Schema
///
/// # Examples
///
/// ```rust,ignore
/// use serde_json::json;
///
/// // Valid JSON Schema
/// let valid_schema = json!({
///     "type": "object",
///     "properties": {
///         "name": {"type": "string"}
///     }
/// });
/// assert!(validate_json_schema_value(&valid_schema).is_ok());
///
/// // Invalid JSON Schema
/// let invalid_schema = json!({"type": "invalid_type"});
/// assert!(validate_json_schema_value(&invalid_schema).is_err());
/// ```
pub fn validate_json_schema_value(parameters: &serde_json::Value) -> Result<(), ValidationError> {
    if !jsonschema::meta::is_valid(parameters) {
        return Err(ValidationError::new("invalid_json_schema"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_json_schema_valid() {
        let valid_schema = r#"
        {
            "type": "object",
            "properties": {
                "name": {
                    "type": "string"
                }
            },
            "required": ["name"]
        }
        "#;

        assert!(validate_json_schema(valid_schema).is_ok());
    }

    #[test]
    fn test_validate_json_schema_invalid_json() {
        let invalid_json = "{ invalid json }";
        let result = validate_json_schema(invalid_json);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_json_schema_invalid_schema() {
        // 使用一个明确无效的 JSON Schema（type 字段值无效）
        let invalid_schema = r#"{"type": "invalid_type"}"#;
        let result = validate_json_schema(invalid_schema);
        assert!(result.is_err());
    }
}
