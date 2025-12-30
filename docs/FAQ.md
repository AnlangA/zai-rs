# 常见问题 (FAQ)

## 通用问题

### Q: 如何获取 API 密钥？
A: 请访问 [智谱 AI 开放平台](https://open.bigmodel.cn/) 注册账号并申请 API 密钥。获取到的 API 密钥格式为 `id.secret`。

### Q: zai-rs 是否免费？
A: zai-rs SDK 本身是开源免费的，但使用智谱 AI API 会产生费用。请查看智谱 AI 的定价页面了解详细信息。

### Q: 最低支持的 Rust 版本是什么？
A: zai-rs 使用 Rust Edition 2024，建议使用最新的稳定版 Rust。运行 `rustup update` 更新您的 Rust 工具链。

### Q: 如何报告 bug 或请求新功能？
A: 请在 [GitHub Issues](https://github.com/AnlangA/zai-rs/issues) 提交 Issue。报告 bug 时请提供重现步骤、错误信息和环境信息。

---

## 安装和配置

### Q: 如何在项目中添加 zai-rs 依赖？
A: 在 `Cargo.toml` 中添加：

```toml
[dependencies]
zai-rs = "0.1"
```

### Q: 如何配置 API 密钥？
A: 最简单的方式是使用环境变量：

```rust
use zai_rs::client::ZaiClient;

// 从环境变量读取
let api_key = std::env::var("ZAI_API_KEY").expect("ZAI_API_KEY must be set");
let client = ZaiClient::new(api_key);
```

或直接在代码中设置（不推荐）：

```rust
let client = ZaiClient::new("your.id.secret".to_string());
```

### Q: API 密钥格式是什么？
A: 智谱 AI API 密钥格式为 `<id>.<secret>`，例如 `abc123.abcdefghijklmnopqrstuvwxyz`。可以使用 `validate_api_key` 函数验证格式：

```rust
use zai_rs::client::error::validate_api_key;

if let Err(e) = validate_api_key(&api_key) {
    eprintln!("Invalid API key: {}", e);
}
```

---

## 使用问题

### Q: 如何进行流式聊天？
A: 使用 `ChatCompletionRequest::streaming()` 方法：

```rust
use zai_rs::model::chat_completion::ChatCompletionRequest;
use futures::StreamExt;

let request = ChatCompletionRequest::new("Hello", "glm-4")
    .streaming(true)
    .build()?;

let mut stream = client.chat_completions_stream(&request).await?;

while let Some(chunk) = stream.next().await {
    println!("{}", chunk?.choices[0].delta.content);
}
```

### Q: 如何处理 API 错误？
A: 所有 API 调用返回 `ZaiResult<T>`。使用 `?` 操作符或 `match` 处理错误：

```rust
match client.chat_completions(&request).await {
    Ok(response) => println!("{:?}", response),
    Err(ZaiError::AuthError { code, message }) => {
        eprintln!("Authentication failed [{}]: {}", code, message);
    },
    Err(ZaiError::RateLimitError { .. }) => {
        eprintln!("Rate limit exceeded, please retry later");
    },
    Err(e) => eprintln!("Error: {}", e),
}
```

### Q: 如何配置重试机制？
A: 使用 `HttpClientConfig` 配置重试策略：

```rust
use zai_rs::client::http::{HttpClientConfig, RetryDelay};
use std::time::Duration;

let config = HttpClientConfig::builder()
    .max_retries(5)
    .timeout(Duration::from_secs(120))
    .retry_delay(RetryDelay::exponential(Duration::from_millis(100), Duration::from_secs(10)))
    .enable_logging(true)
    .build();

let client = ZaiClient::new_with_config(api_key, config);
```

### Q: 如何启用请求日志？
A: 在 `HttpClientConfig` 中启用日志记录：

```rust
let config = HttpClientConfig::builder()
    .enable_logging(true)
    .mask_sensitive_data(true)
    .build();
```

日志会使用 `tracing` 框架输出，需要配置 tracing subscriber。

---

## 错误处理

### Q: 1001 错误码是什么意思？
A: 错误码 1001 表示认证失败，通常是 API 密钥无效或过期。请检查您的 API 密钥是否正确。

### Q: 1301 错误码是什么意思？
A: 错误码 1301 表示速率限制，您已超过 API 调用频率限制。SDK 会自动重试此类错误，您也可以配置更长的重试延迟。

### Q: 如何区分客户端错误和服务端错误？
A: 使用 `is_client_error()` 和 `is_server_error()` 方法：

```rust
if error.is_client_error() {
    eprintln!("Client error - check your request parameters");
} else if error.is_server_error() {
    eprintln!("Server error - please try again later");
}
```

### Q: 如何避免在日志中暴露 API 密钥？
A: SDK 会自动过滤敏感信息。如果需要手动过滤，使用 `mask_sensitive_info` 函数：

```rust
use zai_rs::client::error::mask_sensitive_info;

let log_msg = "Request sent with api_key=abc123.xyz456";
let safe_msg = mask_sensitive_info(log_msg);
// safe_msg: "Request sent with api_key=[FILTERED]"
```

---

## 性能优化

### Q: 如何提高 API 调用性能？
A: 以下是一些优化建议：
1. 启用连接池（默认已启用）
2. 使用流式响应减少延迟
3. 配置适当的超时和重试策略
4. 对于批量操作，考虑使用并发调用

### Q: 如何处理并发请求？
A: 使用 `tokio::spawn` 或 `futures` crate 并发执行：

```rust
use futures::future::join_all;

let tasks = requests.iter()
    .map(|req| client.chat_completions(req))
    .collect::<Vec<_>>();

let results = join_all(tasks).await;
```

### Q: 连接池是如何工作的？
A: SDK 使用 `reqwest::Client` 自动管理连接池。具有相同配置的请求会复用连接，提高性能。

---

## 功能特定问题

### Q: 如何使用工具调用（Function Calling）？
A: 定义工具并在请求中传递：

```rust
use zai_rs::model::tools::{Tool, FunctionTool};

let tool = Tool::Function(
    FunctionTool::new("get_weather")
        .description("Get weather information")
        .parameters(json!({"type": "object", "properties": {"location": {"type": "string"}}}))
);

let request = ChatCompletionRequest::new("What's the weather in Beijing?", "glm-4")
    .tools(vec![tool])
    .build()?;
```

### Q: 如何上传文件？
A: 使用文件上传 API：

```rust
use zai_rs::file::upload_file;
use tokio::fs::File;

let file = File::open("document.pdf").await?;
let result = upload_file(&client, file, Some("application/pdf")).await?;
```

### Q: 如何使用知识库功能？
A: 首先上传文档，然后创建知识库：

```rust
use zai_rs::knowledge::document_upload;

// 上传文档
let file_id = document_upload(&client, file).await?;

// 使用知识库回答问题
let request = ChatCompletionRequest::new("基于文档回答问题", "glm-4")
    .knowledge_base(vec![file_id])
    .build()?;
```

---

## 故障排除

### Q: 连接超时怎么办？
A: 增加超时时间配置：

```rust
let config = HttpClientConfig::builder()
    .timeout(Duration::from_secs(300))
    .build();
```

### Q: 编译时出现 feature 错误怎么办？
A: 某些功能可能需要启用 feature：

```toml
[dependencies]
zai-rs = { version = "0.1", features = ["full"] }
```

### Q: 如何调试请求问题？
A: 启用详细日志：

```rust
let config = HttpClientConfig::builder()
    .enable_logging(true)
    .build();

// 配置 tracing
tracing_subscriber::fmt()
    .with_max_level(tracing::Level::DEBUG)
    .init();
```

---

## 其他

### Q: 是否支持异步/await？
A: 是的，zai-rs 完全基于异步编程模型（tokio），所有 API 都是异步的。

### Q: 如何贡献代码？
A: 欢迎贡献！请先提交 Issue 讨论您的想法，然后提交 Pull Request。确保代码通过 `cargo test` 和 `cargo clippy`。

### Q: 是否有示例代码？
A: 是的，请查看 [examples/](../examples/) 目录，包含各种使用场景的示例代码。

---

**如果您的问题不在这里，请查阅其他文档或在 GitHub 上提问。**
