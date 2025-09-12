use jsonschema;
use validator::ValidationError;

/// 验证 JSON Schema（字符串）的自定义验证函数
pub fn validate_json_schema(parameters: &str) -> Result<(), ValidationError> {
    let schema_json: serde_json::Value =
        serde_json::from_str(parameters).map_err(|_| ValidationError::new("invalid_json"))?;
    if jsonschema::validator_for(&schema_json).is_err() {
        return Err(ValidationError::new("invalid_json_schema"));
    }
    Ok(())
}

/// 验证 JSON Schema（已解析的 Value）的自定义验证函数
pub fn validate_json_schema_value(parameters: &serde_json::Value) -> Result<(), ValidationError> {
    if jsonschema::validator_for(parameters).is_err() {
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
