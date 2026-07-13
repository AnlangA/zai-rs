//! JSON Schema validation for tool parameter definitions.
//!
//! The core build
//! always parses string input and requires a schema-shaped object or boolean.
//! Enabling the `toolkits` feature additionally performs full JSON Schema
//! meta-validation through the `jsonschema` crate.
//!
//! ## Errors
//!
//! Both validation functions return `ValidationError` with specific error
//! codes:
//! - `"invalid_json"` - Input string is not valid JSON
//! - `"invalid_json_schema"` - JSON is valid but not a valid JSON Schema
//!
//! ## Examples
//!
//! ```rust
//! use zai_rs::model::model_validate::{
//!     validate_json_schema, validate_json_schema_value,
//! };
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

use validator::ValidationError;

/// Validate a JSON Schema encoded as a string.
///
/// With the `toolkits` feature enabled this performs meta-schema validation;
/// otherwise it validates the top-level JSON shape.
///
/// # Errors
///
/// * `"invalid_json"` - The input string is not valid JSON
/// * `"invalid_json_schema"` - The JSON is valid but not a valid JSON Schema
///
/// # Examples
///
/// ```rust
/// use zai_rs::model::model_validate::validate_json_schema;
///
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
#[cfg(feature = "toolkits")]
pub fn validate_json_schema(parameters: &str) -> Result<(), ValidationError> {
    let schema_json: serde_json::Value = serde_json::from_str(parameters).map_err(|e| {
        // Preserve the parse error (line/column/reason) in the message instead
        // of discarding it; the error CODE stays "invalid_json" for stability.
        ValidationError::new("invalid_json").with_message(std::borrow::Cow::Owned(format!(
            "invalid JSON in schema: {e}"
        )))
    })?;
    if jsonschema::validator_for(&schema_json).is_err() {
        return Err(ValidationError::new("invalid_json_schema"));
    }
    Ok(())
}

/// Perform lightweight structural validation when `toolkits` is disabled.
#[cfg(not(feature = "toolkits"))]
pub fn validate_json_schema(parameters: &str) -> Result<(), ValidationError> {
    let value: serde_json::Value =
        serde_json::from_str(parameters).map_err(|_| ValidationError::new("invalid_json"))?;
    validate_json_schema_value(&value)
}

/// Validate a parsed JSON Schema value.
///
/// With the `toolkits` feature enabled this performs meta-schema validation;
/// otherwise it requires an object or boolean schema.
///
/// # Errors
///
/// * `"invalid_json_schema"` - The JSON value is not a valid JSON Schema
///
/// # Examples
///
/// ```rust
/// use zai_rs::model::model_validate::validate_json_schema_value;
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
#[cfg(feature = "toolkits")]
pub fn validate_json_schema_value(parameters: &serde_json::Value) -> Result<(), ValidationError> {
    if jsonschema::validator_for(parameters).is_err() {
        return Err(ValidationError::new("invalid_json_schema"));
    }
    Ok(())
}

/// Require a valid top-level JSON Schema shape when `toolkits` is disabled.
///
/// JSON Schema permits an object schema or the boolean schemas `true` and
/// `false`. Keyword-level meta-validation requires the `toolkits` feature.
#[cfg(not(feature = "toolkits"))]
pub fn validate_json_schema_value(parameters: &serde_json::Value) -> Result<(), ValidationError> {
    if parameters.is_object() || parameters.is_boolean() {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_json_schema"))
    }
}

#[cfg(all(test, feature = "toolkits"))]
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
        // An invalid `type` value proves that semantic schema validation runs.
        let invalid_schema = r#"{"type": "invalid_type"}"#;
        let result = validate_json_schema(invalid_schema);
        assert!(result.is_err());
    }
}

#[cfg(all(test, not(feature = "toolkits")))]
mod lightweight_tests {
    use super::*;

    #[test]
    fn validates_json_and_top_level_schema_shape_without_optional_dependency() {
        assert!(validate_json_schema(r#"{"type":"object"}"#).is_ok());
        assert!(validate_json_schema("true").is_ok());
        assert!(validate_json_schema("not json").is_err());
        assert!(validate_json_schema("[]").is_err());
        assert!(validate_json_schema("42").is_err());
    }
}
