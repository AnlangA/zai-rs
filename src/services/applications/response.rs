//! Response types for the 7 LLM-application endpoints (plan P06).
//!
//! All response structs use open schemas — `serde_json::Value` — with
//! `#[serde(default)]` so missing fields decode cleanly.

use serde::Deserialize;

// ---------------------------------------------------------------------------
// 1. ApplicationFileStatsResponse
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct ApplicationFileStatsResponse {
    #[serde(default)]
    pub data: serde_json::Value,
}

// ---------------------------------------------------------------------------
// 2. ApplicationFileUploadResponse
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct ApplicationFileUploadResponse {
    #[serde(default)]
    pub data: serde_json::Value,
}

// ---------------------------------------------------------------------------
// 3. ApplicationSliceInfoResponse
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct ApplicationSliceInfoResponse {
    #[serde(default)]
    pub data: serde_json::Value,
}

// ---------------------------------------------------------------------------
// 4. ApplicationConversationCreateResponse
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct ApplicationConversationCreateResponse {
    #[serde(default)]
    pub data: serde_json::Value,
}

// ---------------------------------------------------------------------------
// 5. ApplicationVariablesResponse
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct ApplicationVariablesResponse {
    #[serde(default)]
    pub data: serde_json::Value,
}

// ---------------------------------------------------------------------------
// 6. ApplicationHistoryResponse
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct ApplicationHistoryResponse {
    #[serde(default)]
    pub data: serde_json::Value,
}

// ---------------------------------------------------------------------------
// 7. ApplicationInvokeResponse
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct ApplicationInvokeResponse {
    #[serde(default)]
    pub data: serde_json::Value,
}
