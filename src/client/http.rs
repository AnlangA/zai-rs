use log::{info, debug};
use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Debug, Deserialize)]
struct ApiErrorEnvelope {
    error: ApiError,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    code: ErrorCode,
    message: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ErrorCode {
    Str(String),
    Num(i64),
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorCode::Str(s) => write!(f, "{}", s),
            ErrorCode::Num(n) => write!(f, "{}", n),
        }
    }
}


// A single shared HTTP client for connection pooling and TLS reuse
static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
fn http_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| reqwest::Client::builder().build().expect("Failed to build reqwest Client"))
}

pub trait HttpClient {
    // Associated types
    type Body: serde::Serialize;
    type ApiUrl: AsRef<str>;
    type ApiKey: AsRef<str>;

    // Accessors
    fn api_url(&self) -> &Self::ApiUrl;
    fn api_key(&self) -> &Self::ApiKey;
    fn body(&self) -> &Self::Body;

    // POST using anyhow for error handling; serialize body here
    fn post(&self) -> impl std::future::Future<Output = anyhow::Result<reqwest::Response>> + Send {
        let body_compact = serde_json::to_string(self.body());
        // Only compute pretty JSON when info-level logging is enabled to avoid extra serialization cost
        let body_pretty_opt = if log::log_enabled!(log::Level::Info) {
            Some(serde_json::to_string_pretty(self.body()).unwrap_or_default())
        } else {
            None
        };
        let url = self.api_url().as_ref().to_owned();
        let key = self.api_key().as_ref().to_owned();
        async move {
            let body = body_compact?;
            if let Some(pretty) = body_pretty_opt {
                info!("Request body: {}", pretty);
            }
            let resp = http_client()
                .post(url)
                .bearer_auth(key)
                .header("Content-Type", "application/json")
                .body(body)
                .send()
                .await?;

            let status = resp.status();
            if status.is_success() {
                return Ok(resp);
            }
            // Debug headers for troubleshooting on non-2xx
            debug!(
                "HTTP {} {} headers: {:?}",
                status.as_u16(),
                status.canonical_reason().unwrap_or(""),
                resp.headers()
            );


            // Non-success HTTP status: parse error JSON and return Err
            let text = resp.text().await.unwrap_or_default();
            if let Ok(parsed) = serde_json::from_str::<ApiErrorEnvelope>(&text) {
                let code_str = parsed.error.code.to_string();
                return Err(anyhow::anyhow!(
                    "HTTP {} {} | code={} | message={}",
                    status.as_u16(),
                    status.canonical_reason().unwrap_or(""),
                    code_str,
                    parsed.error.message
                ));
            } else {
                return Err(anyhow::anyhow!(
                    "HTTP {} {} | body={}",
                    status.as_u16(),
                    status.canonical_reason().unwrap_or(""),
                    text
                ));
            }
        }
    }

    // GET helper (no request body)
    fn get(&self) -> impl std::future::Future<Output = anyhow::Result<reqwest::Response>> + Send {
        let url = self.api_url().as_ref().to_owned();
        let key = self.api_key().as_ref().to_owned();
        async move {
            let resp = http_client()
                .get(url)
                .bearer_auth(key)
                .send()
                .await?;

            let status = resp.status();
            if status.is_success() {
                return Ok(resp);
            }
            // Debug headers for troubleshooting on non-2xx
            debug!(
                "HTTP {} {} headers: {:?}",
                status.as_u16(),
                status.canonical_reason().unwrap_or(""),
                resp.headers()
            );

            // Non-success HTTP status: parse error JSON and return Err
            let text = resp.text().await.unwrap_or_default();
            if let Ok(parsed) = serde_json::from_str::<ApiErrorEnvelope>(&text) {
                let code_str = parsed.error.code.to_string();
                return Err(anyhow::anyhow!(
                    "HTTP {} {} | code={} | message={}",
                    status.as_u16(),
                    status.canonical_reason().unwrap_or(""),
                    code_str,
                    parsed.error.message
                ));
            } else {
                return Err(anyhow::anyhow!(
                    "HTTP {} {} | body={}",
                    status.as_u16(),
                    status.canonical_reason().unwrap_or(""),
                    text
                ));
            }
        }
    }
}
