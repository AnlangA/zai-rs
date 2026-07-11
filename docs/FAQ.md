# 常见问题 (FAQ)

## 通用问题

### Q: 如何获取 API 密钥？
A: 请访问 [智谱 AI 开放平台](https://open.bigmodel.cn/) 注册账号并申请 API 密钥。获取到的 API 密钥格式为 `id.secret`。

### Q: zai-rs 是否免费？
A: zai-rs SDK 本身是开源免费的，但使用智谱 AI API 会产生费用。请查看智谱 AI 的定价页面了解详细信息。

### Q: 最低支持的 Rust 版本是什么？
A: zai-rs 使用 Rust Edition 2024，最低支持 Rust 1.88+。建议使用最新的稳定版 Rust，运行 `rustup update` 更新您的 Rust 工具链。

### Q: 如何报告 bug 或请求新功能？
A: 请在 [GitHub Issues](https://github.com/AnlangA/zai-rs/issues) 提交 Issue。报告 bug 时请提供重现步骤、错误信息和环境信息。

---

## 安装和配置

### Q: 如何在项目中添加 zai-rs 依赖？
A: 在 `Cargo.toml` 中添加：

```toml
[dependencies]
zai-rs = "0.4"
```

### Q: 如何配置 API 密钥？
A: 最简单的方式是使用环境变量：

```rust
use zai_rs::model::*;

// 从环境变量读取
let key = std::env::var("ZHIPU_API_KEY")?;
let client = ChatCompletion::new(GLM4_5_flash {}, TextMessage::user("你好"), key);
```

或直接在代码中设置（不推荐）：

```rust
let client = ChatCompletion::new(
    GLM4_5_flash {},
    TextMessage::user("你好"),
    "your.id.secret",
);
```

### Q: API 密钥格式是什么？
A: 智谱 AI API 密钥格式为 `<id>.<secret>`，例如 `abc123.abcdefghijklmnopqrstuvwxyz`。可以使用 `validate_api_key` 函数验证格式：

```rust
use zai_rs::client::error::validate_api_key;

if let Err(e) = validate_api_key(&api_key) {
    tracing::error!("Invalid API key: {}", e);
}
```

---

## 使用问题

### Q: 如何进行流式聊天？
A: 对 `ChatCompletion` 调用 `enable_stream()`，然后消费 SSE / typed stream：

```rust
use zai_rs::model::*;

let key = std::env::var("ZHIPU_API_KEY")?;
let mut client = ChatCompletion::new(
    GLM4_5_flash {},
    TextMessage::user("讲一个短故事"),
    key,
)
.enable_stream();

client
    .stream_sse_for_each(|data| {
        print!("{}", String::from_utf8_lossy(data));
    })
    .await?;
```

### Q: 如何处理 API 错误？
A: 所有 API 调用返回 `ZaiResult<T>`。使用 `?` 操作符或 `match` 处理错误：

```rust
use zai_rs::{model::*, ZaiError};

let key = std::env::var("ZHIPU_API_KEY")?;
let client = ChatCompletion::new(GLM4_5_flash {}, TextMessage::user("你好"), key);

match client.send().await {
    Ok(response) => println!("{:?}", response),
    Err(ZaiError::AuthError { code, message }) => {
        tracing::error!("Authentication failed [{}]: {}", code, message);
    },
    Err(ZaiError::RateLimitError { .. }) => {
        tracing::error!("Rate limit exceeded, please retry later");
    },
    Err(e) => tracing::error!("Error: {}", e),
}
```

### Q: 如何配置重试机制？
A: 使用 `HttpTransportConfig` 设置最大尝试次数。只有幂等请求会自动重试；POST/PATCH 不会被隐式重放：

```rust
use zai_rs::client::{HttpTransportConfig, ZaiClient};

let config = HttpTransportConfig::builder()
    .max_attempts(2)?
    .build();
let client = ZaiClient::builder(std::env::var("ZHIPU_API_KEY")?)
    .transport(config)
    .build()?;
```

### Q: 如何启用请求日志？
A: 统一传输层使用 `tracing`，配置 subscriber 和 `RUST_LOG` 即可：

```rust
tracing_subscriber::fmt()
    .with_env_filter("zai_rs=debug")
    .init();
```

日志会使用 `tracing` 框架输出，需要配置 tracing subscriber。

---

## 错误处理

### Q: 1001 错误码是什么意思？
A: 错误码 1001 表示认证失败，通常是 API 密钥无效或过期。请检查您的 API 密钥是否正确。

### Q: 1301 错误码是什么意思？
A: 错误码 1301 表示内容安全/策略阻断，请调整输入内容后再请求；SDK 不会自动重试此类错误。频率、并发、配额类错误通常是 1302-1305、1308-1313，SDK 会按重试配置处理。

### Q: 如何区分客户端错误和服务端错误？
A: 使用 `is_client_error()` 和 `is_server_error()` 方法：

```rust
if error.is_client_error() {
    tracing::error!("Client error - check your request parameters");
} else if error.is_server_error() {
    tracing::error!("Server error - please try again later");
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
use zai_rs::model::*;

let key = std::env::var("ZHIPU_API_KEY")?;
let clients = ["问题 1", "问题 2"]
    .into_iter()
    .map(|prompt| ChatCompletion::new(GLM4_5_flash {}, TextMessage::user(prompt), key.clone()));

let results = join_all(clients.map(|client| async move { client.send().await })).await;
```

### Q: 连接池是如何工作的？
A: SDK 使用 `reqwest::Client` 自动管理连接池。具有相同配置的请求会复用连接，提高性能。

---

## 功能特定问题

### Q: 如何使用工具调用（Function Calling）？
A: 定义工具并在请求中传递：

```rust
use zai_rs::model::*;

let key = std::env::var("ZHIPU_API_KEY")?;
let weather = Function::new(
    "get_weather",
    "Get weather information",
    serde_json::json!({
        "type": "object",
        "properties": {"city": {"type": "string"}},
        "required": ["city"]
    }),
);

let client = ChatCompletion::new(
    GLM4_5_flash {},
    TextMessage::user("What's the weather in Beijing?"),
    key,
)
.add_tool(Tools::Function { function: weather });
```

### Q: 如何上传文件？
A: 使用文件上传 API：

```rust
use zai_rs::file::*;

let key = std::env::var("ZHIPU_API_KEY")?;
let result: FileObject = FileUploadRequest::new(
    key,
    FilePurpose::FileExtract,
    "document.pdf",
)
.with_content_type("application/pdf")
.send()
.await?;
```

### Q: 如何使用知识库功能？
A: 对已有知识库上传文档，然后查询知识库详情：

```rust
use zai_rs::knowledge::*;

let key = std::env::var("ZHIPU_API_KEY")?;
let knowledge_id = "your-knowledge-id".to_string();

let upload: UploadFileResponse = DocumentUploadFileRequest::new(key.clone(), knowledge_id.clone())
    .add_file_path("document.pdf")
    .send()
    .await?;

let retrieve: KnowledgeRetrieveResponse = KnowledgeRetrieveRequest::new(key, knowledge_id)
    .send()
    .await?;
```

---

## 故障排除

### Q: 连接超时怎么办？
A: 可以收紧（不能提高）SDK 的连接与单次请求超时上限：

```rust
use zai_rs::client::HttpTransportConfig;

let config = HttpTransportConfig::builder()
    .connect_timeout(Duration::from_secs(5))?
    .request_timeout(Duration::from_secs(30))?
    .build();
```

### Q: 编译时出现 feature 错误怎么办？
A: 某些功能可能需要启用 feature：

```toml
[dependencies]
zai-rs = { version = "0.5", features = ["rmcp-kits"] }
```

### Q: 如何调试请求问题？
A: 配置 `tracing` 详细日志（传输层不会输出鉴权头或 secret）：

```rust
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
