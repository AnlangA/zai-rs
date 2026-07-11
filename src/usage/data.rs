//! Coding Plan usage / quota query client.
//!
//! Wraps the GLM Coding Plan "余量查询" endpoint
//! `GET https://open.bigmodel.cn/api/monitor/usage/quota/limit`. The endpoint is
//! authenticated with a normal Bearer API key and reports the per-5-hour and
//! weekly quota consumption / remaining amounts for the subscribed Coding Plan.
//!
//! See <https://docs.bigmodel.cn/cn/coding-plan/extension/usage-query-plugin>
//! and the community CLI <https://github.com/JinHanAI/coding-plan-monitor>.

use std::fmt;

use chrono::{DateTime, FixedOffset, TimeZone, Utc};
use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    ZaiResult,
    client::{
        http::{HttpClientConfig, parse_typed_response, send_empty_request},
        {ApiFamily, ZaiClient},
    },
};
use std::sync::Arc;

fn de_opt_string_from_string_or_number<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::Null => Ok(None),
        serde_json::Value::String(value) => Ok(Some(value)),
        serde_json::Value::Number(value) => Ok(Some(value.to_string())),
        other => Err(serde::de::Error::custom(format!(
            "expected string or number, got {other}"
        ))),
    }
}

fn parse_next_reset_time(raw: &str) -> Option<DateTime<Utc>> {
    if let Ok(timestamp) = raw.parse::<i64>() {
        return if timestamp.abs() >= 10_000_000_000 {
            Utc.timestamp_millis_opt(timestamp).single()
        } else {
            Utc.timestamp_opt(timestamp, 0).single()
        };
    }

    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|datetime| datetime.with_timezone(&Utc))
}

fn beijing_offset() -> Option<FixedOffset> {
    // UTC+08:00 (8*3600s east) is always a valid fixed offset; UTC (offset 0)
    // is the panic-free fallback. Both are always `Some`, so callers always
    // receive a timezone — the `Option` just keeps this helper panic-free.
    FixedOffset::east_opt(8 * 60 * 60).or_else(|| FixedOffset::east_opt(0))
}

/// `data` payload returned by the quota-limit endpoint.
///
/// `level` is the subscribed plan tier. Depending on the upstream deployment it
/// may be returned as a human-readable tier (`"max"` / `"pro"` / `"lite"`) or
/// a numeric tier code, which is normalized to a string here. `limits` lists
/// each quota window currently in effect for the account.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CodingPlanUsageData {
    /// Subscribed plan level identifier.
    #[serde(
        default,
        deserialize_with = "de_opt_string_from_string_or_number",
        skip_serializing_if = "Option::is_none"
    )]
    pub level: Option<String>,
    /// Active quota windows (5-hour token limit, weekly token limit, …).
    #[serde(default)]
    pub limits: Vec<CodingPlanQuotaLimit>,
}

/// Per-model usage breakdown returned for some quota windows.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CodingPlanUsageDetail {
    /// Model identifier reported by the monitor API.
    #[serde(
        default,
        alias = "modelCode",
        deserialize_with = "de_opt_string_from_string_or_number",
        skip_serializing_if = "Option::is_none"
    )]
    pub model_code: Option<String>,
    /// Amount attributed to this model.
    #[serde(default)]
    pub usage: u64,
}

impl fmt::Display for CodingPlanUsageDetail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {}",
            self.model_code.as_deref().unwrap_or("unknown"),
            self.usage
        )
    }
}

/// Normalized quota-window kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodingPlanQuotaKind {
    /// Per-5-hour Coding Plan quota window.
    TimeLimit,
    /// Weekly token quota window.
    TokensLimit,
    /// Any future or deployment-specific quota window.
    Other(String),
}

impl CodingPlanQuotaKind {
    /// Raw API identifier for this quota kind.
    pub fn as_str(&self) -> &str {
        match self {
            CodingPlanQuotaKind::TimeLimit => "TIME_LIMIT",
            CodingPlanQuotaKind::TokensLimit => "TOKENS_LIMIT",
            CodingPlanQuotaKind::Other(type_) => type_,
        }
    }
}

/// Easy-to-use quota-window view derived from [`CodingPlanQuotaLimit`].
///
/// This hides wire-format quirks such as `currentValue`, numeric `unit`, and
/// millisecond `nextResetTime`, while preserving the raw strings for display or
/// debugging when needed.
#[derive(Debug, Clone)]
pub struct CodingPlanQuotaSummary {
    /// Normalized quota kind.
    pub kind: CodingPlanQuotaKind,
    /// Raw API type string.
    pub type_: String,
    /// Raw API unit normalized to a string.
    pub unit: Option<String>,
    /// Raw `number` value reported by the API.
    pub number: u64,
    /// Raw `usage` value reported by the API, when present.
    pub reported_usage: Option<u64>,
    /// Raw `currentValue` value reported by the API, when present.
    pub current_value: Option<u64>,
    /// Raw `remaining` value reported by the API, when present.
    pub reported_remaining: Option<u64>,
    /// Total quota amount.
    pub quota: u64,
    /// Consumed amount in the current window.
    pub used: u64,
    /// Remaining amount in the current window.
    pub remaining: u64,
    /// Consumed percentage reported by the API.
    pub percentage: f64,
    /// Raw reset value normalized to a string.
    pub next_reset_time: Option<String>,
    /// Parsed reset time in UTC when the raw value is a Unix timestamp or
    /// RFC3339.
    pub next_reset_at: Option<DateTime<Utc>>,
    /// Optional per-model breakdown.
    pub usage_details: Vec<CodingPlanUsageDetail>,
}

fn display_opt<T>(value: Option<T>) -> String
where
    T: fmt::Display,
{
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string())
}

impl CodingPlanQuotaSummary {
    /// True when this is the per-5-hour window.
    pub fn is_time_limit(&self) -> bool {
        self.kind == CodingPlanQuotaKind::TimeLimit
    }

    /// True when this is the weekly tokens window.
    pub fn is_tokens_limit(&self) -> bool {
        self.kind == CodingPlanQuotaKind::TokensLimit
    }

    /// Consumed fraction in `[0.0, 1.0]`.
    pub fn used_ratio(&self) -> f64 {
        if self.quota == 0 {
            return 0.0;
        }

        (self.used as f64 / self.quota as f64).clamp(0.0, 1.0)
    }

    /// Return the usage attributed to one model code, if present.
    pub fn usage_for_model(&self, model_code: &str) -> Option<u64> {
        self.usage_details
            .iter()
            .find(|detail| detail.model_code.as_deref() == Some(model_code))
            .map(|detail| detail.usage)
    }

    /// Parsed reset time in UTC+08:00 (Beijing / Shanghai fixed offset).
    pub fn next_reset_at_beijing(&self) -> Option<DateTime<FixedOffset>> {
        self.next_reset_at
            .as_ref()
            .and_then(|datetime| beijing_offset().map(|tz| datetime.with_timezone(&tz)))
    }
}

impl fmt::Display for CodingPlanQuotaSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}:", self.type_)?;
        writeln!(f, "  kind: {}", self.kind.as_str())?;
        writeln!(f, "  unit: {}", display_opt(self.unit.as_deref()))?;
        writeln!(f, "  number: {}", self.number)?;
        writeln!(f, "  usage: {}", display_opt(self.reported_usage))?;
        writeln!(f, "  current_value: {}", display_opt(self.current_value))?;
        writeln!(
            f,
            "  reported_remaining: {}",
            display_opt(self.reported_remaining)
        )?;
        writeln!(f, "  quota: {}", self.quota)?;
        writeln!(f, "  used: {}", self.used)?;
        writeln!(f, "  remaining: {}", self.remaining)?;
        writeln!(f, "  percentage: {:.1}%", self.percentage)?;
        writeln!(
            f,
            "  next_reset_time: {}",
            display_opt(self.next_reset_time.as_deref())
        )?;
        writeln!(
            f,
            "  next_reset_at_utc: {}",
            display_opt(self.next_reset_at.as_ref().map(DateTime::to_rfc3339))
        )?;
        writeln!(
            f,
            "  next_reset_at_beijing: {}",
            display_opt(
                self.next_reset_at_beijing()
                    .as_ref()
                    .map(DateTime::to_rfc3339)
            )
        )?;

        if self.usage_details.is_empty() {
            writeln!(f, "  usage_details: []")?;
        } else {
            writeln!(f, "  usage_details:")?;
            for detail in &self.usage_details {
                writeln!(f, "    - {detail}")?;
            }
        }

        Ok(())
    }
}

/// Easy-to-use Coding Plan usage view.
#[derive(Debug, Clone, Default)]
pub struct CodingPlanUsageSummary {
    /// Business status code (0 / 200 indicate success).
    pub code: i64,
    /// Human-readable status message.
    pub msg: Option<String>,
    /// Whether the request succeeded.
    pub success: bool,
    /// Subscribed plan level, if reported by the server.
    pub level: Option<String>,
    /// All quota windows as normalized summaries.
    pub limits: Vec<CodingPlanQuotaSummary>,
}

impl CodingPlanUsageSummary {
    /// Find the first quota window matching a raw type string.
    pub fn find_limit(&self, type_: &str) -> Option<&CodingPlanQuotaSummary> {
        self.limits
            .iter()
            .find(|limit| limit.type_.eq_ignore_ascii_case(type_))
    }

    /// The per-5-hour time window, if present.
    pub fn time_limit(&self) -> Option<&CodingPlanQuotaSummary> {
        self.limits
            .iter()
            .find(|limit| limit.kind == CodingPlanQuotaKind::TimeLimit)
    }

    /// The weekly tokens window, if present.
    pub fn tokens_limit(&self) -> Option<&CodingPlanQuotaSummary> {
        self.limits
            .iter()
            .find(|limit| limit.kind == CodingPlanQuotaKind::TokensLimit)
    }
}

impl fmt::Display for CodingPlanUsageSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Coding Plan Usage")?;
        writeln!(f, "code: {}", self.code)?;
        writeln!(f, "msg: {}", display_opt(self.msg.as_deref()))?;
        writeln!(f, "success: {}", self.success)?;
        writeln!(f, "level: {}", display_opt(self.level.as_deref()))?;

        if self.limits.is_empty() {
            writeln!(f, "limits: []")?;
            return Ok(());
        }

        writeln!(f, "limits:")?;
        for (index, limit) in self.limits.iter().enumerate() {
            if index > 0 {
                writeln!(f)?;
            }
            write!(f, "{limit}")?;
        }

        Ok(())
    }
}

/// One quota window reported by the monitor endpoint.
///
/// The Coding Plan applies two kinds of windows, surfaced as `TIME_LIMIT`
/// (per-5-hour) and `TOKENS_LIMIT` (weekly tokens). Depending on the upstream
/// deployment, each window may include explicit `currentValue` / `remaining` /
/// `usage` counters, or only a `number` plus consumed `percentage`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingPlanQuotaLimit {
    /// Limit kind: `TIME_LIMIT` (5-hour window) or `TOKENS_LIMIT` (weekly
    /// tokens).
    #[serde(rename = "type")]
    pub type_: String,
    /// Window unit (e.g. `"5h"`, `"week"`).
    #[serde(
        default,
        deserialize_with = "de_opt_string_from_string_or_number",
        skip_serializing_if = "Option::is_none"
    )]
    pub unit: Option<String>,
    /// Configured cap for this window (tokens or prompt count).
    #[serde(default)]
    pub number: u64,
    /// Consumed percentage within the current window (0–100).
    #[serde(default)]
    pub percentage: f64,
    /// Total quota amount reported by the server, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<u64>,
    /// Amount already consumed in the current window, when present.
    #[serde(
        default,
        alias = "currentValue",
        skip_serializing_if = "Option::is_none"
    )]
    pub current_value: Option<u64>,
    /// Remaining amount reported by the server, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remaining: Option<u64>,
    /// When this window resets (server timestamp / ISO string).
    #[serde(
        default,
        alias = "nextResetTime",
        deserialize_with = "de_opt_string_from_string_or_number",
        skip_serializing_if = "Option::is_none"
    )]
    pub next_reset_time: Option<String>,
    /// Optional per-model breakdown for this quota window.
    #[serde(default, alias = "usageDetails", skip_serializing_if = "Vec::is_empty")]
    pub usage_details: Vec<CodingPlanUsageDetail>,
}

impl CodingPlanQuotaLimit {
    /// Normalized quota kind.
    pub fn kind(&self) -> CodingPlanQuotaKind {
        if self.is_time_limit() {
            CodingPlanQuotaKind::TimeLimit
        } else if self.is_tokens_limit() {
            CodingPlanQuotaKind::TokensLimit
        } else {
            CodingPlanQuotaKind::Other(self.type_.clone())
        }
    }

    /// Total quota amount when the server reports it, falling back to `number`.
    pub fn quota(&self) -> u64 {
        self.usage.unwrap_or(self.number)
    }

    /// Remaining amount in this window.
    ///
    /// Uses the server-provided `remaining` field when present. Otherwise it
    /// falls back to `number * (1 - percentage/100)`.
    pub fn remaining(&self) -> u64 {
        if let Some(remaining) = self.remaining {
            return remaining;
        }
        if let (Some(usage), Some(current_value)) = (self.usage, self.current_value) {
            return usage.saturating_sub(current_value);
        }
        if self.number == 0 {
            return 0;
        }
        let consumed = (self.number as f64) * (self.percentage / 100.0);
        self.number.saturating_sub(consumed.round() as u64)
    }

    /// Remaining fraction in `[0.0, 1.0]`.
    pub fn remaining_ratio(&self) -> f64 {
        if self.number == 0 {
            return 0.0;
        }
        1.0 - (self.percentage / 100.0).clamp(0.0, 1.0)
    }

    /// Consumed amount in this window.
    pub fn consumed(&self) -> u64 {
        if let Some(current_value) = self.current_value {
            return current_value;
        }
        if let (Some(usage), Some(remaining)) = (self.usage, self.remaining) {
            return usage.saturating_sub(remaining);
        }
        let consumed = (self.number as f64) * (self.percentage / 100.0);
        consumed.round() as u64
    }

    /// Alias for [`CodingPlanQuotaLimit::consumed`].
    pub fn used(&self) -> u64 {
        self.consumed()
    }

    /// Parsed reset time in UTC when `next_reset_time` is a Unix timestamp or
    /// RFC3339 datetime.
    pub fn next_reset_at(&self) -> Option<DateTime<Utc>> {
        self.next_reset_time
            .as_deref()
            .and_then(parse_next_reset_time)
    }

    /// Return the usage attributed to one model code, if present.
    pub fn usage_for_model(&self, model_code: &str) -> Option<u64> {
        self.usage_details
            .iter()
            .find(|detail| detail.model_code.as_deref() == Some(model_code))
            .map(|detail| detail.usage)
    }

    /// Parsed reset time in UTC+08:00 (Beijing / Shanghai fixed offset).
    pub fn next_reset_at_beijing(&self) -> Option<DateTime<FixedOffset>> {
        self.next_reset_at()
            .and_then(|datetime| beijing_offset().map(|tz| datetime.with_timezone(&tz)))
    }

    /// Build an easy-to-use normalized view for this quota window.
    pub fn summary(&self) -> CodingPlanQuotaSummary {
        CodingPlanQuotaSummary {
            kind: self.kind(),
            type_: self.type_.clone(),
            unit: self.unit.clone(),
            number: self.number,
            reported_usage: self.usage,
            current_value: self.current_value,
            reported_remaining: self.remaining,
            quota: self.quota(),
            used: self.used(),
            remaining: self.remaining(),
            percentage: self.percentage,
            next_reset_time: self.next_reset_time.clone(),
            next_reset_at: self.next_reset_at(),
            usage_details: self.usage_details.clone(),
        }
    }

    /// True when this is the per-5-hour time window.
    pub fn is_time_limit(&self) -> bool {
        self.type_.eq_ignore_ascii_case("TIME_LIMIT")
    }

    /// True when this is the weekly tokens window.
    pub fn is_tokens_limit(&self) -> bool {
        self.type_.eq_ignore_ascii_case("TOKENS_LIMIT")
    }
}

impl fmt::Display for CodingPlanQuotaLimit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.summary().fmt(f)
    }
}

/// Standard `{code, msg, success, data}` envelope used by the monitor API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingPlanUsageResponse {
    /// Business status code (0 / 200 indicate success).
    #[serde(default)]
    pub code: i64,
    /// Human-readable status message.
    #[serde(
        default,
        rename = "msg",
        deserialize_with = "de_opt_string_from_string_or_number",
        skip_serializing_if = "Option::is_none"
    )]
    pub msg: Option<String>,
    /// Whether the request succeeded.
    #[serde(default)]
    pub success: bool,
    /// Quota payload.
    #[serde(default)]
    pub data: CodingPlanUsageData,
}

impl CodingPlanUsageResponse {
    /// The active quota windows.
    pub fn limits(&self) -> &[CodingPlanQuotaLimit] {
        &self.data.limits
    }

    /// Find the first window matching a limit type (case-insensitive).
    pub fn find_limit(&self, type_: &str) -> Option<&CodingPlanQuotaLimit> {
        self.data
            .limits
            .iter()
            .find(|l| l.type_.eq_ignore_ascii_case(type_))
    }

    /// The per-5-hour time window, if present.
    pub fn time_limit(&self) -> Option<&CodingPlanQuotaLimit> {
        self.find_limit("TIME_LIMIT")
    }

    /// The weekly tokens window, if present.
    pub fn tokens_limit(&self) -> Option<&CodingPlanQuotaLimit> {
        self.find_limit("TOKENS_LIMIT")
    }

    /// The subscribed plan level, if reported by the server.
    pub fn level(&self) -> Option<&str> {
        self.data.level.as_deref()
    }

    /// Build an easy-to-use normalized view of the quota response.
    pub fn summary(&self) -> CodingPlanUsageSummary {
        CodingPlanUsageSummary {
            code: self.code,
            msg: self.msg.clone(),
            success: self.success,
            level: self.data.level.clone(),
            limits: self
                .data
                .limits
                .iter()
                .map(CodingPlanQuotaLimit::summary)
                .collect(),
        }
    }

    /// The per-5-hour window as a normalized summary, if present.
    pub fn time_limit_summary(&self) -> Option<CodingPlanQuotaSummary> {
        self.time_limit().map(CodingPlanQuotaLimit::summary)
    }

    /// The weekly tokens window as a normalized summary, if present.
    pub fn tokens_limit_summary(&self) -> Option<CodingPlanQuotaSummary> {
        self.tokens_limit().map(CodingPlanQuotaLimit::summary)
    }
}

impl fmt::Display for CodingPlanUsageResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.summary().fmt(f)
    }
}

/// Coding Plan usage / quota query request
/// (`GET /api/monitor/usage/quota/limit`) (P05: routes through [`ZaiClient`]).
///
/// Construct with [`CodingPlanUsageRequest::new`] and execute with
/// [`CodingPlanUsageRequest::send_via`]. Credentials and transport live on the
/// [`ZaiClient`], passed to `send_via`.
///
/// ```rust,no_run
/// use zai_rs::usage::CodingPlanUsageRequest;
/// use zai_rs::client::ZaiClient;
///
/// # async fn go(client: ZaiClient) -> zai_rs::ZaiResult<()> {
/// let resp = CodingPlanUsageRequest::new().send_via(&client).await?;
/// if let Some(five_hour) = resp.time_limit() {
///     tracing::info!("5h window: {}% used", five_hour.percentage);
/// }
/// # Ok(())
/// # }
/// ```
pub struct CodingPlanUsageRequest {
    _body: (),
}

impl CodingPlanUsageRequest {
    /// Build a quota query. The base URL, credentials and transport are
    /// supplied by the [`ZaiClient`] passed to [`Self::send_via`].
    pub fn new() -> Self {
        Self { _body: () }
    }

    /// Send the quota query via a [`ZaiClient`] and parse the typed envelope.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<CodingPlanUsageResponse> {
        let url = client
            .endpoints()
            .resolve(ApiFamily::Monitor, &["usage", "quota", "limit"])?;
        let config = transport_config_from_client(client);
        let resp = send_empty_request(
            reqwest::Method::GET,
            url,
            client.secret().expose(),
            Arc::new(config),
        )
        .await?;
        parse_typed_response::<CodingPlanUsageResponse>(resp).await
    }
}

impl Default for CodingPlanUsageRequest {
    fn default() -> Self {
        Self::new()
    }
}

fn transport_config_from_client(client: &ZaiClient) -> HttpClientConfig {
    let t = client.transport();
    HttpClientConfig {
        timeout: std::time::Duration::from_secs(t.request_timeout.as_secs()),
        max_retries: u32::from(t.max_attempts).saturating_sub(1),
        enable_compression: t.enable_compression,
        retry_delay: crate::client::http::RetryDelay::Exponential {
            base: std::time::Duration::from_millis(500),
            max: std::time::Duration::from_secs(5),
        },
        enable_logging: false,
        mask_sensitive_data: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_typical_envelope() {
        let raw = r#"{
            "code": 200,
            "msg": "ok",
            "success": true,
            "data": {
                "level": "max",
                "limits": [
                    {
                        "type": "TIME_LIMIT",
                        "unit": "5h",
                        "number": 600,
                        "percentage": 25.0,
                        "nextResetTime": "2026-06-18T15:00:00Z"
                    },
                    {
                        "type": "TOKENS_LIMIT",
                        "unit": "week",
                        "number": 1000000,
                        "percentage": 50.0,
                        "nextResetTime": "2026-06-22T00:00:00Z"
                    }
                ]
            }
        }"#;
        let resp: CodingPlanUsageResponse = serde_json::from_str(raw).unwrap();
        assert!(resp.success);
        assert_eq!(resp.level(), Some("max"));
        let t = resp.time_limit().unwrap();
        assert!(t.is_time_limit());
        assert_eq!(t.consumed(), 150);
        assert_eq!(t.remaining(), 450);
        assert!((t.remaining_ratio() - 0.75).abs() < 1e-9);
        assert_eq!(t.next_reset_time.as_deref(), Some("2026-06-18T15:00:00Z"));
        assert_eq!(
            t.next_reset_at().unwrap().to_rfc3339(),
            "2026-06-18T15:00:00+00:00"
        );
        let w = resp.tokens_limit().unwrap();
        assert!(w.is_tokens_limit());
        assert_eq!(w.consumed(), 500_000);
        assert_eq!(w.remaining(), 500_000);
    }

    #[test]
    fn deserializes_numeric_plan_level() {
        let raw = r#"{
            "code": 200,
            "msg": "ok",
            "success": true,
            "data": {
                "level": 3,
                "limits": [
                    {
                        "type": "TIME_LIMIT",
                        "unit": 5,
                        "number": 600,
                        "percentage": 25.0,
                        "usage": 4000,
                        "currentValue": 54,
                        "remaining": 3946,
                        "nextResetTime": 1781778751996,
                        "usageDetails": [
                            {"modelCode": "search-prime", "usage": 40},
                            {"modelCode": "web-reader", "usage": 14}
                        ]
                    }
                ]
            }
        }"#;

        let resp: CodingPlanUsageResponse = serde_json::from_str(raw).unwrap();

        assert_eq!(resp.level(), Some("3"));
        let limit = resp.time_limit().unwrap();
        assert_eq!(limit.unit.as_deref(), Some("5"));
        assert_eq!(limit.quota(), 4000);
        assert_eq!(limit.consumed(), 54);
        assert_eq!(limit.remaining(), 3946);
        assert_eq!(limit.usage_details.len(), 2);
        assert_eq!(
            limit.usage_details[0].model_code.as_deref(),
            Some("search-prime")
        );
        assert_eq!(limit.usage_for_model("web-reader"), Some(14));
        assert_eq!(limit.next_reset_time.as_deref(), Some("1781778751996"));

        let summary = resp.summary();
        assert_eq!(summary.code, 200);
        assert_eq!(summary.msg.as_deref(), Some("ok"));
        assert!(summary.success);
        assert_eq!(summary.level.as_deref(), Some("3"));
        let time_limit = summary.time_limit().unwrap();
        assert_eq!(time_limit.kind, CodingPlanQuotaKind::TimeLimit);
        assert_eq!(time_limit.number, 600);
        assert_eq!(time_limit.reported_usage, Some(4000));
        assert_eq!(time_limit.current_value, Some(54));
        assert_eq!(time_limit.reported_remaining, Some(3946));
        assert_eq!(time_limit.quota, 4000);
        assert_eq!(time_limit.used, 54);
        assert_eq!(time_limit.remaining, 3946);
        assert_eq!(time_limit.usage_for_model("search-prime"), Some(40));
        assert_eq!(
            time_limit.next_reset_at.as_ref().unwrap().to_rfc3339(),
            "2026-06-18T10:32:31.996+00:00"
        );
        assert_eq!(
            time_limit
                .next_reset_at_beijing()
                .as_ref()
                .unwrap()
                .to_rfc3339(),
            "2026-06-18T18:32:31.996+08:00"
        );

        let report = summary.to_string();
        assert!(report.contains("code: 200"));
        assert!(report.contains("msg: ok"));
        assert!(report.contains("success: true"));
        assert!(report.contains("number: 600"));
        assert!(report.contains("current_value: 54"));
        assert!(report.contains("reported_remaining: 3946"));
        assert!(report.contains("next_reset_at_utc: 2026-06-18T10:32:31.996+00:00"));
        assert!(report.contains("next_reset_at_beijing: 2026-06-18T18:32:31.996+08:00"));
        assert!(report.contains("usage_details:"));
        assert!(report.contains("search-prime: 40"));
    }

    #[test]
    fn handles_missing_optional_fields() {
        let raw = r#"{"code":0,"success":true,"data":{"limits":[]}}"#;
        let resp: CodingPlanUsageResponse = serde_json::from_str(raw).unwrap();
        assert!(resp.limits().is_empty());
        assert!(resp.level().is_none());
        assert!(resp.time_limit().is_none());
    }

    #[test]
    fn remaining_is_zero_when_exhausted() {
        let limit = CodingPlanQuotaLimit {
            type_: "TIME_LIMIT".to_string(),
            unit: Some("5h".to_string()),
            number: 100,
            percentage: 100.0,
            usage: None,
            current_value: None,
            remaining: None,
            next_reset_time: None,
            usage_details: Vec::new(),
        };
        assert_eq!(limit.remaining(), 0);
        assert_eq!(limit.consumed(), 100);
    }

    #[test]
    fn monitor_family_resolves_official_endpoint() {
        let ec = crate::client::endpoint::EndpointConfig::defaults().unwrap();
        let url = ec
            .resolve(
                crate::client::ApiFamily::Monitor,
                &["usage", "quota", "limit"],
            )
            .unwrap();
        assert_eq!(
            url,
            "https://open.bigmodel.cn/api/monitor/usage/quota/limit"
        );
    }

    #[test]
    fn monitor_family_resolves_custom_base_via_builder() {
        use crate::client::endpoint::{ApiFamily, EndpointConfig};
        let ec = EndpointConfig::builder()
            .monitor("https://api.z.ai/api/monitor")
            .build(false)
            .unwrap();
        let url = ec
            .resolve(ApiFamily::Monitor, &["usage", "quota", "limit"])
            .unwrap();
        assert_eq!(url, "https://api.z.ai/api/monitor/usage/quota/limit");
    }
}
