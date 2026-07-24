# 最佳实践

以下约定适用于长期运行、并发调用 API 的服务端应用。

## 1. API 密钥管理

### 使用环境变量
```rust,ignore
// 从 ZHIPU_API_KEY 构造共享客户端。
let client = ZaiClient::from_env()?;

// 不要在源码中硬编码真实密钥。
let client = ZaiClient::builder("example-id.example-secret-value").build()?;
```

### 使用 `.env` 文件
```dotenv
ZHIPU_API_KEY=example-id.example-secret-value
```

`.env` 不是 SDK 自动加载功能；如需使用，请由应用选择并配置相应的 dotenv
库，然后仍通过 `ZaiClient::from_env()` 创建客户端。

### 验证 API 密钥格式
```rust,ignore
use zai_rs::client::error::validate_api_key;

validate_api_key(&api_key)?;
let client = ZaiClient::builder(api_key).build()?;
```

## 2. 错误处理

### 使用 `Result` 类型
```rust,ignore
async fn chat_with_client(client: &ZaiClient, prompt: &str) -> ZaiResult<String> {
    let response = ChatCompletion::new(GLM4_5_flash {}, TextMessage::user(prompt))
        .send_via(client)
        .await?;
    Ok(response
        .choices()
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.message())
        .and_then(|message| message.content_str())
        .unwrap_or_default()
        .to_owned())
}

// 对比示例：不要在生产请求路径中使用 unwrap。
async fn chat_bad(client: &ZaiClient, prompt: &str) -> String {
    let response = ChatCompletion::new(GLM4_5_flash {}, TextMessage::user(prompt))
        .send_via(client)
        .await
        .unwrap();
    response.choices().unwrap()[0]
        .message()
        .unwrap()
        .content_str()
        .unwrap()
        .to_owned()
}
```

### 区分错误类型
```rust,ignore
match result {
    Ok(response) => { /* 处理成功 */ },
    Err(ZaiError::AuthError { .. }) => {
        // 提示用户检查 API 密钥
    },
    Err(ZaiError::RateLimitError { .. }) => {
        // 提示用户稍后重试
    },
    Err(ZaiError::NetworkError(_)) => {
        // 网络错误，可能需要检查连接
    },
    Err(e) => {
        // 其他错误
        tracing::error!("Unexpected error: {}", e);
    },
}
```

### 使用重试机制

`max_attempts` 包含首次请求。自动重试只适用于幂等 HTTP 方法；普通
`POST/PATCH` 不会被隐式重放。

```rust,ignore
use zai_rs::client::{HttpTransportConfig, ZaiClient};
use std::time::Duration;

let config = HttpTransportConfig::builder()
    .max_attempts(3)?
    .request_timeout(Duration::from_secs(60))?
    .build();
let client = ZaiClient::builder(api_key).transport(config).build()?;
```

## 3. 请求优化

### 启用压缩
统一传输层默认启用 gzip，并由 `ZaiClient` 持有的单一连接池透明解压响应；请求对象不会各自创建客户端。

### 设置合适的超时

单次请求默认 60 秒。普通 JSON 调用通常应保持默认或收紧；只有预期的大文件慢速
传输才应显式提高，配置上限为 24 小时。

```rust,ignore
let config = HttpTransportConfig::builder()
    .request_timeout(Duration::from_secs(30))?
    .build();
```

### 使用类型安全的流式响应
```rust,ignore
let mut stream = ChatCompletion::new(GLM4_5_flash {}, TextMessage::user(prompt))
    .with_max_tokens(1_000)
    .enable_stream()
    .stream_via(&client)
    .await?;

while let Some(chunk) = stream.next().await {
    let chunk = chunk?;
    if let Some(text) = chunk
        .choices
        .first()
        .and_then(|choice| choice.delta.as_ref())
        .and_then(|delta| delta.content.as_deref())
    {
        print!("{text}");
    }
}
```

旧版文档中的 `chat_completions_stream`、`sse_stream` 和
`stream_sse_for_each` 已删除；当前入口统一为 `stream_via(&client)`。

## 4. 日志和监控

### 配置日志
```rust,ignore
use tracing_subscriber;

tracing_subscriber::fmt()
    .with_max_level(tracing::Level::INFO)
    .init();

// ZaiClient 的 Debug 输出会隐藏密钥；应用自己的日志仍需自行脱敏。
```

### 结构化日志
```rust,ignore
use tracing::{info, error, instrument};

#[instrument(skip(client))]
async fn process_request(client: &ZaiClient, user_id: &str) -> ZaiResult<()> {
    info!(user_id, "Processing request");
    let request = ChatCompletion::new(
        GLM4_5_flash {},
        TextMessage::user("health summary"),
    )
    .with_user_id(user_id);

    match request.send_via(client).await {
        Ok(response) => {
            info!(
                tokens_used = response
                    .usage()
                    .and_then(|usage| usage.total_tokens())
                    .unwrap_or_default(),
                "Request completed"
            );
            Ok(())
        },
        Err(e) => {
            error!(error = %e, user_id, "Request failed");
            Err(e)
        },
    }
}
```

### 过滤敏感信息
```rust,ignore
use zai_rs::client::error::mask_sensitive_info;

// 仅在必须记录外部文本时兜底清理；不要主动把 key 拼进日志。
let safe_msg = mask_sensitive_info(external_message);
```

## 5. 并发和性能

### 使用连接池
```rust,ignore
// 克隆不会复制密钥内容或创建新的连接池。
let client2 = client.clone();
```

### 并发请求
```rust,ignore
use tokio::task::JoinSet;

let mut tasks = JoinSet::new();
for prompt in prompts {
    let client = client.clone();
    tasks.spawn(async move { chat_with_client(&client, &prompt).await });
}

while let Some(result) = tasks.join_next().await {
    match result? {
        Ok(content) => println!("{content}"),
        Err(error) => tracing::error!(%error, "chat failed"),
    }
}
```

### 限制并发数量
```rust,ignore
use tokio::sync::Semaphore;
use std::sync::Arc;

let semaphore = Arc::new(Semaphore::new(5));
let mut tasks = JoinSet::new();

for prompt in prompts {
    let client = client.clone();
    let semaphore = semaphore.clone();
    tasks.spawn(async move {
        let _permit = semaphore.acquire_owned().await.map_err(|_| ZaiError::ApiError {
            code: zai_rs::client::error::codes::SDK_VALIDATION,
            message: "concurrency limiter closed".to_owned(),
        })?;
        chat_with_client(&client, &prompt).await
    });
}

while let Some(result) = tasks.join_next().await {
    let _response = result??;
}
```

## 6. 内存和资源管理

### 控制模型响应规模
```rust,ignore
let request = ChatCompletion::new(GLM4_5_flash {}, TextMessage::user(prompt))
    .with_max_tokens(1_000);
let response = request.send_via(&client).await?;
process_response(&response);
```

## 7. 测试

使用本地 HTTP mock server 构造真实 `ZaiClient` 端点，断言请求方法、路径、请求体
和错误映射。仓库内可参考 `tests/support/http_server.rs` 与
`tests/send_via_coverage.rs`。

## 8. 安全性

### 使用 HTTPS
SDK 默认使用 HTTPS，无需额外配置。

### 验证输入

请求 builder 会校验冻结 API 契约中的长度、枚举和组合约束。应用仍应在构造请求前
执行自己的授权、内容策略和资源配额检查；不要编造与服务端契约无关的统一长度上限。

### 使用安全的日志级别

生产环境通常使用 `INFO`；无论日志级别如何，都不要记录原始密钥、Authorization
header 或未脱敏的用户内容。

## 9. 代码组织

### 模块化代码
```rust,ignore
// src/chat.rs
pub mod chat {
    use zai_rs::{ZaiClient, ZaiResult, model::*};

    pub async fn ask(client: &ZaiClient, prompt: &str) -> ZaiResult<String> {
        let response = ChatCompletion::new(GLM4_5_flash {}, TextMessage::user(prompt))
            .send_via(client)
            .await?;
        Ok(response
            .choices()
            .and_then(|choices| choices.first())
            .and_then(|choice| choice.message())
            .and_then(|message| message.content_str())
            .unwrap_or_default()
            .to_owned())
    }
}

// 使用
use crate::chat::chat;

let answer = chat::ask(&client, "Hello").await?;
```

### 创建辅助函数
```rust,ignore
// 封装常用配置
fn create_client() -> Result<ZaiClient, Box<dyn std::error::Error>> {
    let api_key = std::env::var("ZHIPU_API_KEY")?;
    validate_api_key(&api_key)?;

    let config = HttpTransportConfig::builder()
        .max_attempts(3)?
        .request_timeout(Duration::from_secs(60))?
        .build();

    Ok(ZaiClient::builder(api_key).transport(config).build()?)
}
```

## 10. 生产环境配置

### 环境变量管理
```bash
# .env.production
ZHIPU_API_KEY=prod-id.example-production-secret
RUST_LOG=info

# .env.development
ZHIPU_API_KEY=dev-id.example-development-secret
RUST_LOG=debug
```

### 监控和告警
```rust,ignore
// 记录关键指标
use tracing::{Instrument, info, info_span};

let request = ChatCompletion::new(
    GLM4_5_flash {},
    TextMessage::user(prompt),
);

let span = info_span!("api_call", model = "glm-4.5-flash", prompt_len = prompt.len());

async {
    let result = request.send_via(&client).await;
    if let Ok(resp) = &result {
        info!(
            tokens_used = resp
                .usage()
                .and_then(|usage| usage.total_tokens())
                .unwrap_or_default(),
            "API call completed"
        );
    }
    result
}
.instrument(span)
.await
```

### 健康检查

进程存活探针不应调用付费模型 API。就绪探针可检查本地配置是否已加载；若业务确实
需要端到端探测，应使用低频、独立配额并明确计费和限流影响。
