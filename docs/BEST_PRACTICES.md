# 最佳实践

本指南提供使用 zai-rs SDK 的最佳实践建议，帮助您构建健壮、高效的应用程序。

---

## 1. API 密钥管理

### 使用环境变量
```rust
// ✅ 推荐：从环境变量读取
let api_key = std::env::var("ZAI_API_KEY")
    .expect("ZAI_API_KEY environment variable must be set");
let client = ZaiClient::new(api_key);

// ❌ 避免：硬编码 API 密钥
let client = ZaiClient::new("your.hardcoded.key".to_string());
```

### 使用 `.env` 文件
```rust
// .env 文件
ZAI_API_KEY=your.api.key.here

// 代码中加载
dotenv::dotenv().ok();
let api_key = std::env::var("ZAI_API_KEY")?;
```

### 验证 API 密钥格式
```rust
use zai_rs::client::error::validate_api_key;

validate_api_key(&api_key)?;
let client = ZaiClient::new(api_key);
```

---

## 2. 错误处理

### 使用 `Result` 类型
```rust
// ✅ 推荐：使用 ? 操作符
async fn chat_with_client(client: &ZaiClient, prompt: &str) -> ZaiResult<String> {
    let request = ChatCompletionRequest::new(prompt, "glm-4").build()?;
    let response = client.chat_completions(&request).await?;
    Ok(response.choices[0].message.content.clone())
}

// ❌ 避免：使用 unwrap
async fn chat_bad(client: &ZaiClient, prompt: &str) -> String {
    let request = ChatCompletionRequest::new(prompt, "glm-4").build().unwrap();
    let response = client.chat_completions(&request).await.unwrap();
    response.choices[0].message.content.clone()
}
```

### 区分错误类型
```rust
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
        eprintln!("Unexpected error: {}", e);
    },
}
```

### 使用重试机制
```rust
use zai_rs::client::http::{HttpClientConfig, RetryDelay};
use std::time::Duration;

let config = HttpClientConfig::builder()
    .max_retries(3)
    .timeout(Duration::from_secs(120))
    .retry_delay(RetryDelay::exponential(
        Duration::from_millis(500),
        Duration::from_secs(10),
    ))
    .build();

let client = ZaiClient::new_with_config(api_key, config);
```

---

## 3. 请求优化

### 启用压缩
```rust
let config = HttpClientConfig::builder()
    .compression(true)  // 默认启用
    .build();
```

### 设置合适的超时
```rust
// 快速请求
let config = HttpClientConfig::builder()
    .timeout(Duration::from_secs(30))
    .build();

// 长时间处理任务
let config = HttpClientConfig::builder()
    .timeout(Duration::from_secs(300))
    .build();
```

### 使用流式响应
```rust
// ✅ 推荐：流式响应提供实时反馈
let request = ChatCompletionRequest::new(prompt, "glm-4")
    .streaming(true)
    .build()?;

let mut stream = client.chat_completions_stream(&request).await?;

while let Some(chunk) = stream.next().await {
    print!("{}", chunk?.choices[0].delta.content);
    std::io::stdout().flush()?;
}

// ❌ 避免：等待完整响应
let response = client.chat_completions(&request).await?;
println!("{}", response.choices[0].message.content);
```

---

## 4. 日志和监控

### 配置日志
```rust
use tracing_subscriber;

tracing_subscriber::fmt()
    .with_max_level(tracing::Level::INFO)
    .init();

let config = HttpClientConfig::builder()
    .enable_logging(true)
    .mask_sensitive_data(true)
    .build();
```

### 结构化日志
```rust
use tracing::{info, error, instrument};

#[instrument(skip(client))]
async fn process_request(client: &ZaiClient, user_id: &str) -> ZaiResult<()> {
    info!(user_id, "Processing request");

    match client.chat_completions(&request).await {
        Ok(response) => {
            info!(tokens_used = response.usage.total_tokens, "Request completed");
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
```rust
use zai_rs::client::error::mask_sensitive_info;

let log_msg = format!("Request with key: {}", api_key);
let safe_msg = mask_sensitive_info(&log_msg);
// safe_msg: "Request with key: [FILTERED]"
```

---

## 5. 并发和性能

### 使用连接池
```rust
// SDK 内部使用 reqwest::Client 自动管理连接池
// 相同配置的请求会复用连接

// 为不同配置创建不同的客户端
let config1 = HttpClientConfig::builder().timeout(Duration::from_secs(30)).build();
let config2 = HttpClientConfig::builder().timeout(Duration::from_secs(60)).build();

let client1 = ZaiClient::new_with_config(api_key.clone(), config1);
let client2 = ZaiClient::new_with_config(api_key, config2);
```

### 并发请求
```rust
use futures::future::join_all;

// 批量处理
let tasks = prompts.iter()
    .map(|p| chat_with_client(&client, p))
    .collect::<Vec<_>>();

let results = join_all(tasks).await;

for result in results {
    match result {
        Ok(content) => println!("{}", content),
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

### 限制并发数量
```rust
use tokio::sync::Semaphore;
use std::sync::Arc;

let semaphore = Arc::new(Semaphore::new(5)); // 最多 5 个并发请求
let mut handles = vec![];

for prompt in prompts {
    let client = client.clone();
    let semaphore = semaphore.clone();
    let handle = tokio::spawn(async move {
        let _permit = semaphore.acquire().await.unwrap();
        chat_with_client(&client, &prompt).await
    });
    handles.push(handle);
}
```

---

## 6. 内存和资源管理

### 使用流式处理大文件
```rust
// ✅ 推荐：流式处理
let mut stream = client.chat_completions_stream(&request).await?;

while let Some(chunk) = stream.next().await {
    // 逐块处理，避免加载全部内容到内存
    process_chunk(chunk?);
}

// ❌ 避免：一次性加载大响应
let response = client.chat_completions(&request).await?;
let full_content = response.choices[0].message.content.clone(); // 可能很大
```

### 限制响应大小
```rust
let request = ChatCompletionRequest::new(prompt, "glm-4")
    .max_tokens(1000)  // 限制生成的 token 数量
    .build()?;
```

### 及时释放资源
```rust
// 使用 drop 显式释放
{
    let response = client.chat_completions(&request).await?;
    process_response(&response);
} // response 在此处被释放
```

---

## 7. 测试

### 使用 Mock 进行测试
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_chat_completion() {
        let mock_client = create_mock_client();
        let result = chat_with_client(&mock_client, "test").await;
        assert!(result.is_ok());
    }
}
```

### 测试错误处理
```rust
#[tokio::test]
async fn test_rate_limit_handling() {
    let mock_client = create_mock_client_with_rate_limit();
    let result = chat_with_client(&mock_client, "test").await;

    assert!(matches!(
        result,
        Err(ZaiError::RateLimitError { .. })
    ));
}
```

---

## 8. 安全性

### 使用 HTTPS
SDK 默认使用 HTTPS，无需额外配置。

### 验证输入
```rust
// ✅ 推荐：验证用户输入
if prompt.len() > 10000 {
    return Err(ZaiError::ApiError {
        code: 1200,
        message: "Prompt too long".to_string(),
    });
}

// ❌ 避免：直接使用未验证的输入
let request = ChatCompletionRequest::new(prompt, "glm-4").build()?;
```

### 使用安全的日志级别
```rust
// 生产环境使用 INFO 级别
tracing_subscriber::fmt()
    .with_max_level(tracing::Level::INFO)
    .init();

// 调试环境使用 DEBUG 级别（敏感信息会被过滤）
let config = HttpClientConfig::builder()
    .enable_logging(cfg!(debug_assertions))
    .mask_sensitive_data(true)
    .build();
```

---

## 9. 代码组织

### 模块化代码
```rust
// src/chat.rs
pub mod chat {
    use zai_rs::client::ZaiClient;
    use zai_rs::model::chat_completion::ChatCompletionRequest;

    pub async fn ask(client: &ZaiClient, prompt: &str) -> ZaiResult<String> {
        let request = ChatCompletionRequest::new(prompt, "glm-4").build()?;
        let response = client.chat_completions(&request).await?;
        Ok(response.choices[0].message.content.clone())
    }
}

// 使用
use crate::chat::chat;

let answer = chat::ask(&client, "Hello").await?;
```

### 创建辅助函数
```rust
// 封装常用配置
fn create_client() -> ZaiResult<ZaiClient> {
    let api_key = std::env::var("ZAI_API_KEY")?;
    validate_api_key(&api_key)?;

    let config = HttpClientConfig::builder()
        .max_retries(3)
        .timeout(Duration::from_secs(120))
        .build();

    Ok(ZaiClient::new_with_config(api_key, config))
}
```

---

## 10. 生产环境配置

### 环境变量管理
```bash
# .env.production
ZAI_API_KEY=prod.api.key
RUST_LOG=info

# .env.development
ZAI_API_KEY=dev.api.key
RUST_LOG=debug
```

### 监控和告警
```rust
// 记录关键指标
use tracing::info_span;

let span = info_span!("api_call", model = "glm-4", prompt_len = prompt.len());

async {
    let result = client.chat_completions(&request).await;
    if let Ok(resp) = &result {
        info!(
            tokens_used = resp.usage.total_tokens,
            cost = calculate_cost(resp.usage.total_tokens),
            "API call completed"
        );
    }
    result
}
.instrument(span)
.await
```

### 健康检查
```rust
async fn health_check(client: &ZaiClient) -> bool {
    let request = ChatCompletionRequest::new("test", "glm-4")
        .max_tokens(10)
        .build();

    client.chat_completions(&request).await.is_ok()
}
```

---

## 总结

1. **安全性优先**：使用环境变量管理密钥，过滤敏感日志
2. **健壮的错误处理**：正确处理所有可能的错误情况
3. **性能优化**：使用流式响应、连接池、并发请求
4. **良好的日志**：配置合适的日志级别，使用结构化日志
5. **模块化设计**：封装常用功能，保持代码清晰
6. **测试覆盖**：为关键功能编写测试

遵循这些最佳实践，可以帮助您构建安全、高效、可维护的应用程序。
