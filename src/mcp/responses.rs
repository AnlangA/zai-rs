//! Typed responses and MCP content decoding.

use std::{collections::BTreeMap, fmt, ops::Deref};

use rmcp::model::CallToolResult;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::{
    ZaiResult,
    client::error::{ZaiError, codes, mask_sensitive_info},
};

const MCP_ERROR_MESSAGE_CHARS_MAX: usize = 1024;

/// Text returned by repository and vision tools.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpTextResponse {
    text: String,
}

impl McpTextResponse {
    pub(crate) fn new(text: String) -> Self {
        Self { text }
    }

    /// Borrow the returned text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Consume the response and return its text.
    pub fn into_text(self) -> String {
        self.text
    }
}

impl Deref for McpTextResponse {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.text()
    }
}

impl fmt::Display for McpTextResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.text)
    }
}

/// One result returned by web search.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebSearchResult {
    /// Result title.
    pub title: String,
    /// Result URL.
    pub link: String,
    /// Search-generated page summary.
    pub content: String,
    /// Upstream reference identifier, when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refer: Option<String>,
    /// Source site or publisher name, when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media: Option<String>,
    /// Source-site icon URL, when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Publication date in the upstream service's original representation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publish_date: Option<String>,
    /// Additional fields returned by newer versions of the upstream service.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Typed response from `web_search_prime`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebSearchResponse {
    /// Search results in upstream ranking order.
    pub results: Vec<WebSearchResult>,
}

impl WebSearchResponse {
    /// Consume the response and return the result list.
    pub fn into_results(self) -> Vec<WebSearchResult> {
        self.results
    }
}

impl Deref for WebSearchResponse {
    type Target = [WebSearchResult];

    fn deref(&self) -> &Self::Target {
        &self.results
    }
}

/// Typed response from `webReader`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebReaderResponse {
    /// Page title, or an empty string when the upstream response omits it.
    #[serde(default)]
    pub title: String,
    /// Page description, or an empty string when unavailable.
    #[serde(default)]
    pub description: String,
    /// Canonical or requested page URL reported by the reader.
    #[serde(default)]
    pub url: String,
    /// Extracted page content in the requested format.
    #[serde(default)]
    pub content: String,
    /// Image labels mapped to their source URLs.
    #[serde(default)]
    pub images: BTreeMap<String, String>,
    /// Reader metadata whose shape is controlled by the upstream service.
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
    /// Additional externally sourced reader data.
    #[serde(default)]
    pub external: BTreeMap<String, Value>,
    /// Unknown top-level fields retained for forward compatibility.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

pub(crate) fn text_response(result: CallToolResult) -> ZaiResult<McpTextResponse> {
    extract_text(result).map(McpTextResponse::new)
}

pub(crate) fn web_search_response(result: CallToolResult) -> ZaiResult<WebSearchResponse> {
    let results = decode_json_text::<Vec<WebSearchResult>>(result)?;
    Ok(WebSearchResponse { results })
}

pub(crate) fn web_reader_response(result: CallToolResult) -> ZaiResult<WebReaderResponse> {
    decode_json_text(result)
}

fn decode_json_text<T: DeserializeOwned>(result: CallToolResult) -> ZaiResult<T> {
    check_tool_error(&result)?;
    if let Some(structured) = result.structured_content {
        return Ok(serde_json::from_value(structured)?);
    }
    let text = collect_text(&result)?;
    let text = decode_string_layer(text);
    Ok(serde_json::from_str(&text)?)
}

fn extract_text(result: CallToolResult) -> ZaiResult<String> {
    check_tool_error(&result)?;
    if let Some(structured) = result.structured_content {
        return match structured {
            Value::String(text) => Ok(text),
            other => Ok(serde_json::to_string(&other)?),
        };
    }
    collect_text(&result).map(decode_string_layer)
}

fn collect_text(result: &CallToolResult) -> ZaiResult<String> {
    let mut texts = result
        .content
        .iter()
        .filter_map(|content| content.as_text().map(|text| text.text.as_str()));
    let first = texts.next().ok_or_else(|| ZaiError::Unknown {
        code: codes::SDK_EXTERNAL_TOOL,
        message: "MCP tool response did not contain text content".to_owned(),
    })?;
    let mut text = String::from(first);
    for additional in texts {
        text.push('\n');
        text.push_str(additional);
    }
    Ok(text)
}

fn decode_string_layer(text: String) -> String {
    match serde_json::from_str::<Value>(&text) {
        Ok(Value::String(decoded)) => decoded,
        _ => text,
    }
}

fn check_tool_error(result: &CallToolResult) -> ZaiResult<()> {
    if result.is_error != Some(true) {
        return Ok(());
    }
    let message = collect_bounded_error_text(result)
        .or_else(|| {
            result
                .structured_content
                .as_ref()
                .and_then(structured_error_message)
                .map(safe_tool_error_message)
        })
        .filter(|message| !message.trim().is_empty())
        .unwrap_or_else(|| "MCP tool returned an error without safe text content".to_owned());
    Err(ZaiError::Unknown {
        code: codes::SDK_EXTERNAL_TOOL,
        message,
    })
}

fn collect_bounded_error_text(result: &CallToolResult) -> Option<String> {
    // Error content is untrusted and can be much larger than a useful
    // diagnostic. Collect at most one extra character so truncation is
    // detectable without first duplicating the complete provider response.
    let collection_limit = MCP_ERROR_MESSAGE_CHARS_MAX + 1;
    let mut collected = String::new();
    let mut collected_chars = 0;
    let mut found_text = false;

    for text in result
        .content
        .iter()
        .filter_map(|content| content.as_text().map(|text| text.text.as_str()))
    {
        if found_text && collected_chars < collection_limit {
            collected.push('\n');
            collected_chars += 1;
        }
        found_text = true;
        for character in text.chars() {
            if collected_chars == collection_limit {
                break;
            }
            collected.push(character);
            collected_chars += 1;
        }
        if collected_chars == collection_limit {
            break;
        }
    }

    if !found_text {
        return None;
    }
    let decoded = if collected_chars <= MCP_ERROR_MESSAGE_CHARS_MAX {
        decode_string_layer(collected)
    } else {
        collected
    };
    Some(safe_tool_error_message(&decoded))
}

fn structured_error_message(value: &Value) -> Option<&str> {
    match value {
        Value::String(message) => Some(message),
        Value::Object(object) => object
            .get("message")
            .and_then(Value::as_str)
            .or_else(|| object.get("error").and_then(Value::as_str))
            .or_else(|| {
                object
                    .get("error")
                    .and_then(Value::as_object)
                    .and_then(|error| error.get("message"))
                    .and_then(Value::as_str)
            }),
        _ => None,
    }
}

fn safe_tool_error_message(message: &str) -> String {
    let redacted = mask_sensitive_info(message);
    let mut characters = redacted.chars();
    let mut bounded: String = characters
        .by_ref()
        .take(MCP_ERROR_MESSAGE_CHARS_MAX)
        .collect();
    if characters.next().is_some() {
        bounded.push('…');
    }
    bounded
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn result(value: Value) -> CallToolResult {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn unwraps_json_encoded_text_content() {
        let value = json!({
            "content": [{"type": "text", "text": "\"hello\""}],
            "isError": false
        });
        assert_eq!(text_response(result(value)).unwrap().text(), "hello");
    }

    #[test]
    fn surfaces_mcp_error_text_as_sdk_error() {
        let value = json!({
            "content": [{"type": "text", "text": "invalid input"}],
            "isError": true
        });
        let error = text_response(result(value)).unwrap_err();
        assert!(error.to_string().contains("invalid input"));
    }

    #[test]
    fn parses_captured_search_encoding() {
        let value = json!({
            "content": [{
                "type": "text",
                "text": "\"[{\\\"title\\\":\\\"Docs\\\",\\\"link\\\":\\\"https://docs.rs\\\",\\\"content\\\":\\\"Rust docs\\\",\\\"refer\\\":\\\"ref_1\\\"}]\""
            }],
            "isError": false
        });
        let response = web_search_response(result(value)).unwrap();
        assert_eq!(response[0].title, "Docs");
        assert_eq!(response[0].refer.as_deref(), Some("ref_1"));
    }

    #[test]
    fn parses_captured_reader_encoding() {
        let value = json!({
            "content": [{
                "type": "text",
                "text": "\"{\\\"title\\\":\\\"Page\\\",\\\"url\\\":\\\"https://example.com\\\",\\\"content\\\":\\\"Body\\\",\\\"images\\\":{},\\\"metadata\\\":{},\\\"external\\\":{}}\""
            }],
            "isError": false
        });
        let response = web_reader_response(result(value)).unwrap();
        assert_eq!(response.title, "Page");
        assert_eq!(response.content, "Body");
    }

    #[test]
    fn structured_content_has_priority() {
        let value = json!({
            "content": [{"type": "text", "text": "ignored"}],
            "structuredContent": [{
                "title": "Structured",
                "link": "https://example.com",
                "content": "Body"
            }],
            "isError": false
        });
        let response = web_search_response(result(value)).unwrap();
        assert_eq!(response[0].title, "Structured");
    }

    #[test]
    fn structured_error_never_becomes_success() {
        let value = json!({
            "structuredContent": {"message": "quota exhausted"},
            "isError": true
        });
        let error = text_response(result(value)).unwrap_err();
        assert!(error.to_string().contains("quota exhausted"));
    }

    #[test]
    fn mcp_error_diagnostics_are_bounded_and_credential_redacted() {
        let secret = "abc123.abcdefghijklmnopqrstuvwxyz";
        let value = json!({
            "content": [{
                "type": "text",
                "text": format!("Authorization: Bearer {secret}; {}", "x".repeat(2_000))
            }],
            "isError": true
        });
        let error = text_response(result(value)).unwrap_err();
        let message = error.message();

        assert!(!message.contains(secret));
        assert!(!message.contains("abcdefghijklmnopqrstuvwxyz"));
        assert!(message.contains("[AUTH_REDACTED]"));
        assert!(message.chars().count() <= MCP_ERROR_MESSAGE_CHARS_MAX + 1);
    }

    #[test]
    fn structured_mcp_error_uses_only_a_safe_message_field() {
        let secret = "abc123.abcdefghijklmnopqrstuvwxyz";
        let value = json!({
            "structuredContent": {
                "message": format!("api_key={secret}"),
                "request": {"prompt": "must not be serialized into the error"}
            },
            "isError": true
        });
        let error = text_response(result(value)).unwrap_err();
        let message = error.message();

        assert!(!message.contains(secret));
        assert!(!message.contains("abcdefghijklmnopqrstuvwxyz"));
        assert!(!message.contains("must not be serialized"));
        assert!(message.contains("[FILTERED]"));
    }

    #[test]
    fn accepts_direct_json_and_unknown_response_fields() {
        let search = json!({
            "content": [{
                "type": "text",
                "text": "[{\"title\":\"Docs\",\"link\":\"https://docs.rs\",\"content\":\"Body\",\"icon\":\"icon.png\",\"futureField\":42}]"
            }]
        });
        let response = web_search_response(result(search)).unwrap();
        assert_eq!(response[0].icon.as_deref(), Some("icon.png"));
        assert_eq!(response[0].extra.get("futureField"), Some(&json!(42)));

        let reader = json!({
            "content": [{
                "type": "text",
                "text": "{\"title\":\"Page\",\"content\":\"Body\",\"links\":[\"https://example.com\"]}"
            }]
        });
        let response = web_reader_response(result(reader)).unwrap();
        assert_eq!(
            response.extra.get("links").unwrap()[0],
            "https://example.com"
        );
    }

    #[test]
    fn joins_multiple_text_blocks_in_order() {
        let value = json!({
            "content": [
                {"type": "text", "text": "first"},
                {"type": "image", "data": "AA==", "mimeType": "image/png"},
                {"type": "text", "text": "second"}
            ]
        });
        assert_eq!(
            text_response(result(value)).unwrap().text(),
            "first\nsecond"
        );
    }

    #[test]
    fn missing_text_is_an_error() {
        let value = json!({"content": [], "isError": false});
        assert!(text_response(result(value)).is_err());
    }
}
