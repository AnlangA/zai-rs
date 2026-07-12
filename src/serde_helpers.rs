//! Crate-private Serde adapters shared by independent API modules.

use serde::{Deserialize, Deserializer};

/// Deserialize an optional identifier that upstream may encode as either a
/// JSON string or number.
pub(crate) fn optional_string_from_number_or_string<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::Null => Ok(None),
        serde_json::Value::String(value) => Ok(Some(value)),
        serde_json::Value::Number(value) => Ok(Some(value.to_string())),
        other => Err(serde::de::Error::custom(format!(
            "expected string, number, or null; got {other}"
        ))),
    }
}

/// Deserialize an optional JSON-encoded string while tolerating providers
/// that send the decoded JSON value directly.
pub(crate) fn optional_json_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::Null => Ok(None),
        serde_json::Value::String(value) => Ok(Some(value)),
        other => serde_json::to_string(&other)
            .map(Some)
            .map_err(serde::de::Error::custom),
    }
}
