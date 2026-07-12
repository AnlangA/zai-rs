# 错误处理指南

`zai-rs` 的所有请求都返回 `ZaiResult<T>`。错误统一为 `ZaiError`，既保留
HTTP 或业务错误码，也提供适合恢复策略的分类方法。

## 基本用法

优先用 `?` 把错误交给调用方，只在应用边界决定日志、重试或用户提示：

```rust,ignore
use zai_rs::{
    ZaiClient,
    client::error::{ZaiError, ZaiResult},
    model::{ChatCompletion, GLM4_5_flash, TextMessage},
};

async fn request(client: &ZaiClient) -> ZaiResult<()> {
    let response = ChatCompletion::new(
        GLM4_5_flash {},
        TextMessage::user("Hello"),
    )
    .send_via(client)
    .await?;

    if let Some(text) = response
        .choices()
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.message())
        .and_then(|message| message.content_str())
    {
        println!("{text}");
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    let result: ZaiResult<()> = async {
        let client = ZaiClient::from_env()?;
        request(&client).await
    }
    .await;

    match result {
        Ok(()) => {},
        Err(ZaiError::AuthError { code, message }) => {
            tracing::error!(code, %message, "认证失败");
        },
        Err(error) if error.is_rate_limit() => {
            tracing::warn!(error = %error.compact(), "请求受到限流");
        },
        Err(error) => tracing::error!(error = %error, "请求失败"),
    }
}
```

应用代码通常不需要匹配所有变体。`category()`、`is_auth_error()`、
`is_rate_limit()`、`is_client_error()`、`is_server_error()` 和
`is_retryable()` 更适合稳定的恢复逻辑。`ZaiError` 是 `#[non_exhaustive]`，
直接匹配时应保留兜底分支。

## 错误类型

| 变体 | 含义 |
|------|------|
| `HttpError` | 未映射为专用类型的 HTTP 状态错误 |
| `AuthError` | HTTP 401/403 或认证类业务错误 |
| `AccountError` | 账户、套餐或余额错误 |
| `ApiError` | 参数/API 错误；也承载部分 SDK 端校验错误 |
| `RateLimitError` | HTTP 429 或限流、配额类业务错误 |
| `ContentPolicyError` | 内容安全或策略拦截 |
| `FileError` | 文件 API 或本地文件校验错误 |
| `NetworkError` | HTTP 网络或超时错误 |
| `JsonError` | JSON 序列化或反序列化错误 |
| `RealtimeError` | WebSocket 连接、协议、超时或关闭错误 |
| `RealtimeAuthError` | Realtime API key/JWT 错误 |
| `Unknown` | 未知业务码或未分类状态 |

`error.code()` 返回可用的 HTTP/业务/SDK 错误码，`error.message()` 返回描述。
SDK 自身使用保留的 `9000..=9999` 错误码；可通过 `is_sdk_error()` 区分它们
与服务端的 `1000..=1499` 业务码。

## 自动重试

统一传输层默认最多尝试 3 次，但只自动重试可安全重放的幂等请求。普通 POST
不会因为启用了重试策略而被重复提交。可恢复结果包括部分网络错误、HTTP 429、
部分 5xx 以及对应的限流业务码；认证、参数、账户、内容策略和文件错误不会重试。

重试采用带 full jitter 的指数退避，并尊重正整数秒格式的 `Retry-After`。
每次尝试和整个请求都有截止时间。可显式收紧策略：

```rust,ignore
use std::time::Duration;
use zai_rs::client::{HttpTransportConfig, ZaiClient};

let transport = HttpTransportConfig::builder()
    .connect_timeout(Duration::from_secs(5))?
    .request_timeout(Duration::from_secs(30))?
    .max_attempts(2)?
    .build();

let client = ZaiClient::builder(api_key)
    .transport(transport)
    .build()?;
# Ok::<(), zai_rs::ZaiError>(())
```

若要在应用层重试非幂等操作，必须先确认服务端提供幂等键或任务去重语义；不要仅凭
`is_retryable()` 就重复提交创建类请求。

## 日志与敏感信息

`ZaiClient` 和内部 secret 的 `Debug`/`Display` 实现不会输出 API key。应用自己
拼接的字符串仍需主动清理：

```rust,ignore
use zai_rs::client::error::{contains_sensitive_info, mask_sensitive_info};

let line = "Authorization: Bearer abc.defghijklmnop";
assert!(contains_sensitive_info(line));
let safe = mask_sensitive_info(line);
assert!(!safe.contains("abcdefghijklmnop"));
```

推荐记录 endpoint 模板、HTTP 状态、错误分类和请求 ID；不要记录 Authorization
header、完整请求体、用户文件内容或 Realtime token。

## 业务码映射

| 代码范围 | 错误类型 |
|----------|----------|
| `1000`, `1001`, `1003`, `1005`, `1220` | `AuthError` |
| `1110..=1121`（`1113` 除外） | `AccountError` |
| `1200..=1234`, `1261` | `ApiError`（其中 `1200/1230/1234` 按服务端故障分类） |
| `1301` | `ContentPolicyError` |
| `1113`, `1302`, `1305`, `1308..=1311`, `1313..=1321` | `RateLimitError` |
| `1400..=1499` | `FileError` |

未知业务码保留为 `Unknown`，不会被假定为可重试。完整定义以
[`src/client/error.rs`](../src/client/error.rs) 和上游 API 文档为准。
