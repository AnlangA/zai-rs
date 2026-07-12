//! Typed responses and MCP content decoding.

use std::{collections::BTreeMap, fmt, ops::Deref};

use rmcp::model::CallToolResult;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::{
    ZaiResult,
    client::error::{ZaiError, codes},
};

/// Text returned by repository and vision tools.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpTextResponse {
    text: String,
}

impl McpTextResponse {
    pub(crate) fn new(text: String) -> Self {
        Self { text }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

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
    pub title: String,
    pub link: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publish_date: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Typed response from `web_search_prime`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebSearchResponse {
    pub results: Vec<WebSearchResult>,
}

impl WebSearchResponse {
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
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub images: BTreeMap<String, String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
    #[serde(default)]
    pub external: BTreeMap<String, Value>,
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
    let message = collect_text(result)
        .or_else(|_| {
            result
                .structured_content
                .as_ref()
                .map(serde_json::to_string)
                .transpose()?
                .ok_or_else(|| ZaiError::Unknown {
                    code: codes::SDK_EXTERNAL_TOOL,
                    message: "MCP tool returned an error without content".to_owned(),
                })
        })
        .unwrap_or_else(|error| error.to_string());
    Err(ZaiError::Unknown {
        code: codes::SDK_EXTERNAL_TOOL,
        message: decode_string_layer(message),
    })
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
