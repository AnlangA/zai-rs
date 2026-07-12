//! Response types for the seven LLM-application endpoints.
//!
//! All response structs use open schemas — `serde_json::Value` — with
//! `#[serde(default)]` so missing fields decode cleanly.

use serde::Deserialize;

/// Open-schema response from the application file-statistics endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct ApplicationFileStatsResponse {
    /// Endpoint-specific response data.
    #[serde(default)]
    pub data: serde_json::Value,
}

/// Open-schema response from the application file-upload endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct ApplicationFileUploadResponse {
    /// Endpoint-specific response data.
    #[serde(default)]
    pub data: serde_json::Value,
}

/// Open-schema response from the application slice-information endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct ApplicationSliceInfoResponse {
    /// Endpoint-specific response data.
    #[serde(default)]
    pub data: serde_json::Value,
}

/// Open-schema response after creating an application conversation.
#[derive(Debug, Clone, Deserialize)]
pub struct ApplicationConversationCreateResponse {
    /// Endpoint-specific response data.
    #[serde(default)]
    pub data: serde_json::Value,
}

/// Open-schema response containing application variables.
#[derive(Debug, Clone, Deserialize)]
pub struct ApplicationVariablesResponse {
    /// Endpoint-specific response data.
    #[serde(default)]
    pub data: serde_json::Value,
}

/// Open-schema response containing application conversation history.
#[derive(Debug, Clone, Deserialize)]
pub struct ApplicationHistoryResponse {
    /// Endpoint-specific response data.
    #[serde(default)]
    pub data: serde_json::Value,
}

/// Open-schema response from an application invocation.
#[derive(Debug, Clone, Deserialize)]
pub struct ApplicationInvokeResponse {
    /// Endpoint-specific response data.
    #[serde(default)]
    pub data: serde_json::Value,
}
