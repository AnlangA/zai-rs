# 错误处理指南

本指南详细说明 `zai-rs` SDK 中的错误处理机制和最佳实践。

## 错误类型

SDK 定义了全面的错误类型，覆盖所有可能的错误场景：

### ZaiError 枚举

| 错误类型 | 说明 | 示例场景 |
|----------|------|----------|
| `HttpError` | HTTP 状态错误 | 400 Bad Request, 404 Not Found |
| `AuthError` | 认证和授权错误 | 无效的 API 密钥 |
| `AccountError` | 账户相关错误 | 账户已过期、余额不足 |
| `ApiError` | API 调用错误 | 无效的参数 |
| `RateLimitError` | 速率限制错误 | 请求过于频繁 |
| `ContentPolicyError` | 内容策略违规 | 违规内容 |
| `FileError` | 文件处理错误 | 文件上传失败 |
| `NetworkError` | 网络/IO 错误 | 连接超时 |
| `JsonError` | JSON 解析错误 | 响应格式无效 |
| `Unknown` | 未知错误 | 未分类的错误 |

## 基础错误处理

### 简单匹配

```rust,ignore
use zai_rs::client::error::ZaiError;
use zai_rs::model::*;

#[tokio::main]
async fn main() {
    let model = GLM4_5_flash {};
    let messages = TextMessage::user("Hello");
    let key = std::env::var("ZHIPU_API_KEY").unwrap();
    
    let client = ChatCompletion::new(model, messages, key);
    
    match client.post().await {
        Ok(resp) => println!("Success: {}", resp.choices().first().unwrap().message.content()),
        Err(ZaiError::AuthError { code, message }) => {
            eprintln!("认证失败 [{}]: {}", code, message);
        }
        Err(ZaiError::RateLimitError { code, message }) => {
            eprintln!("速率限制 [{}]: {}", code, message);
        }
        Err(e) => eprintln!("错误: {}", e),
    }
}
```

### 使用辅助方法

SDK 提供了有用的辅助方法来检查错误类型：

```rust,ignore
use zai_rs::client::error::ZaiError;

fn handle_error(error: &ZaiError) {
    if error.is_rate_limit() {
        eprintln!("触发速率限制，请稍后重试");
    } else if error.is_auth_error() {
        eprintln!("认证失败，请检查 API 密钥");
    } else if error.is_client_error() {
        eprintln!("客户端错误: {}", error.compact());
    } else if error.is_server_error() {
        eprintln!("服务器错误，请稍后重试");
    }
}
```

## 重试机制

SDK 内置了智能重试机制，自动处理临时性错误：

### 自动重试的失败场景

- 服务器错误（5xx）：内部服务器错误、网关超时等
- 速率限制错误：API 代码 1301
- 网络错误：连接超时、连接拒绝

### 不会重试的失败场景

- 客户端错误（4xx）：无效请求、未授权等
- 认证错误：API 密钥无效
- 账户错误：账户不存在、权限不足

### 默认重试配置

```rust
// 默认配置
max_retries: 3
retry_delay: Exponential {
    base: 500ms,
    max: 5s
}
```

### 自定义重试行为

虽然当前版本不支持自定义重试配置，但您可以在应用层实现自己的重试逻辑：

```rust,ignore
use zai_rs::client::error::{ZaiError, ZaiResult};
use tokio::time::{sleep, Duration};
use std::time::Instant;

async fn call_with_retry<F, Fut, T>(
    mut f: F,
    max_retries: u32,
    base_delay: Duration,
) -> ZaiResult<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = ZaiResult<T>>,
{
    let mut last_error = None;
    
    for attempt in 0..=max_retries {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                last_error = Some(e.clone());
                if attempt < max_retries {
                    let delay = base_delay * 2_u32.pow(attempt);
                    eprintln!("尝试 {}/{} 失败，等待 {:?}", attempt + 1, max_retries + 1, delay);
                    sleep(delay).await;
                }
            }
        }
    }
    
    Err(last_error.unwrap())
}

// 使用示例
#[tokio::main]
async fn main() -> ZaiResult<()> {
    let model = GLM4_5_flash {};
    let messages = TextMessage::user("Hello");
    let key = std::env::var("ZHIPU_API_KEY")?;
    
    let result = call_with_retry(
        || {
            let client = ChatCompletion::new(model.clone(), messages.clone(), key.clone());
            client.post()
        },
        5,
        Duration::from_millis(1000),
    ).await?;
    
    Ok(())
}
```

## 日志安全

### 敏感信息过滤

SDK 提供了 `mask_sensitive_info` 函数，用于在日志中过滤敏感信息：

```rust,ignore
use zai_rs::client::error::mask_sensitive_info;

fn log_request(api_key: &str, content: &str) {
    let log_text = format!("API key: {}, Content: {}", api_key, content);
    let filtered = mask_sensitive_info(&log_text);
    
    // 输出: API key: [FILTERED], Content: ...
    println!("{}", filtered);
}
```

### 过滤的内容

- API 密钥（格式：id.secret）
- 密码字段
- Token 值
- Bearer 认证头
- Authorization 头

## 错误代码映射

SDK 将智谱AI API 错误代码映射到特定的错误类型：

### HTTP 状态码

| 状态码 | 错误类型 | 说明 |
|--------|----------|------|
| 400 | `HttpError` | 错误的请求 |
| 401 | `HttpError` | 未授权 - 检查 API 密钥 |
| 404 | `HttpError` | 资源未找到 |
| 429 | `HttpError` | 请求过多 - 速率限制 |
| 434 | `HttpError` | 无 API 权限 |
| 435 | `HttpError` | 文件大小超过 100MB 限制 |
| 500-599 | `HttpError` | 服务器错误 |

### API 业务错误代码

| 代码范围 | 错误类型 | 说明 |
|----------|----------|------|
| 1000-1004, 1100 | `AuthError` | 认证相关错误 |
| 1110-1121 | `AccountError` | 账户相关错误 |
| 1200-1234 | `ApiError` | API 调用错误 |
| 1300-1309 | `RateLimitError` | 速率限制错误 |

更多错误代码详情请参考 [智谱AI API文档](https://docs.bigmodel.cn/cn/api/api-code)。

## 最佳实践

### 1. 始终使用 Result 类型

```rust,ignore
// ✅ 好的做法
async fn get_response() -> ZaiResult<String> {
    let client = ChatCompletion::new(model, messages, key)?;
    let resp = client.post().await?;
    Ok(resp.choices().first()?.message.content())
}

// ❌ 不好的做法
async fn get_response() -> String {
    let resp = client.post().await.unwrap();
    resp.choices().first().unwrap().message.content()
}
```

### 2. 使用 ? 运算符简化错误传播

```rust,ignore
// ✅ 好的做法
async fn process() -> ZaiResult<()> {
    let resp = make_request().await?;
    let content = extract_content(&resp)?;
    Ok(())
}

// ❌ 不好的做法
async fn process() -> ZaiResult<()> {
    let resp = match make_request().await {
        Ok(r) => r,
        Err(e) => return Err(e),
    };
    // ...
}
```

### 3. 提供有意义的错误信息

```rust,ignore
use zai_rs::client::error::{ZaiError, ZaiResult};

async fn get_weather(city: &str) -> ZaiResult<String> {
    let model = GLM4_5_flash {};
    let messages = TextMessage::user(format!("{}的天气如何？", city));
    let key = std::env::var("ZHIPU_API_KEY")
        .map_err(|_| ZaiError::ApiError {
            code: 1200,
            message: "ZHIPU_API_KEY 环境变量未设置".to_string(),
        })?;
    
    let client = ChatCompletion::new(model, messages, key);
    let resp = client.post().await?;
    
    Ok(resp.choices().first().unwrap().message.content())
}

#[tokio::main]
async fn main() {
    match get_weather("北京").await {
        Ok(weather) => println!("天气: {}", weather),
        Err(e) => eprintln!("获取天气失败: {}", e),
    }
}
```

### 4. 使用结构化日志

```rust,ignore
use log::{error, info, warn};

async fn process_request() -> ZaiResult<String> {
    info!("开始处理请求");
    
    let result = make_request().await?;
    
    info!("请求成功");
    Ok(result)
}

#[tokio::main]
async fn main() {
    env_logger::init();
    
    match process_request().await {
        Ok(content) => println!("{}", content),
        Err(e) => error!("处理失败: {}", e.compact()),
    }
}
```

### 5. 验证 API 密钥

```rust,ignore
use zai_rs::client::error::validate_api_key;

fn main() {
    let key = std::env::var("ZHIPU_API_KEY").unwrap();
    
    if let Err(e) = validate_api_key(&key) {
        eprintln!("API 密钥格式错误: {}", e);
        std::process::1);
    }
    
    // 继续正常流程
}
```

## 调试技巧

### 启用详细日志

```rust,ignore
fn main() {
    // 设置日志级别为 DEBUG
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
        .init();
    
    // 运行应用...
}
```

### 检查错误详细信息

```rust,ignore
use zai_rs::client::error::ZaiError;

fn debug_error(error: &ZaiError) {
    println!("错误代码: {:?}", error.code());
    println!("错误信息: {}", error.message());
    println!("紧凑表示: {}", error.compact());
    
    if error.is_rate_limit() {
        println!("这是一个速率限制错误");
    }
}
```

## 相关资源

- [API 错误代码参考](https://docs.bigmodel.cn/cn/api/api-code)
- [快速入门指南](GETTING_STARTED.md)
- [高级主题](ADVANCED_TOPICS.md)
