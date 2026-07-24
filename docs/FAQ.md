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
zai-rs = "6.0.1"
```

### Q: 如何配置 API 密钥？
A: 最简单的方式是使用环境变量：

```rust,ignore
use zai_rs::ZaiClient;

// 从 ZHIPU_API_KEY 读取
let client = ZaiClient::from_env()?;
```

或直接在代码中设置（不推荐）：

```rust,ignore
let client = ZaiClient::builder("example-id.example-secret-value").build()?;
```

### Q: API 密钥格式是什么？
A: 智谱 AI API 密钥格式为 `<id>.<secret>`，例如 `abc123.abcdefghijklmnopqrstuvwxyz`。可以使用 `validate_api_key` 函数验证格式：

```rust,ignore
use zai_rs::client::error::validate_api_key;

if let Err(e) = validate_api_key(&api_key) {
    tracing::error!("Invalid API key: {}", e);
}
```

---

## 使用问题

### Q: 如何进行流式聊天？
A: 调用 `enable_stream()` 后，通过 `stream_via(&client)` 消费强类型 SSE
chunk：

```rust,ignore
use zai_rs::{ZaiClient, model::*};

let client = ZaiClient::from_env()?;
let mut stream = ChatCompletion::new(
    GLM4_5_flash {},
    TextMessage::user("讲一个短故事"),
)
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

### Q: 如何处理 API 错误？
A: 所有 API 调用返回 `ZaiResult<T>`。使用 `?` 操作符或 `match` 处理错误：

```rust,ignore
use zai_rs::{model::*, ZaiClient, ZaiError};

let client = ZaiClient::from_env()?;
let request = ChatCompletion::new(GLM4_5_flash {}, TextMessage::user("你好"));

match request.send_via(&client).await {
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

```rust,ignore
use zai_rs::client::{HttpTransportConfig, ZaiClient};

let config = HttpTransportConfig::builder()
    .max_attempts(2)?
    .build();
let client = ZaiClient::builder(std::env::var("ZHIPU_API_KEY")?)
    .transport(config)
    .build()?;
```

### Q: 如何接收 SDK 的 tracing 事件？
A: 应用可以配置 `tracing` subscriber；SDK 只在已有埋点处产生事件，并不保证记录
每一次请求或重试：

```rust,ignore
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
A: 错误码 1301 表示内容安全/策略阻断，请调整输入内容后再请求；SDK 不会自动重试
此类错误。自动重试还取决于 HTTP 方法、状态码和业务码，不能仅凭错误码范围判断。

### Q: 如何区分客户端错误和服务端错误？
A: 使用 `is_client_error()` 和 `is_server_error()` 方法：

```rust,ignore
if error.is_client_error() {
    tracing::error!("Client error - check your request parameters");
} else if error.is_server_error() {
    tracing::error!("Server error - please try again later");
}
```

### Q: 如何避免在日志中暴露 API 密钥？
A: `ZaiClient` 的 Debug 输出及 SDK 自身鉴权路径不会输出明文密钥，但 SDK 无法过滤
应用自行拼接的任意日志。记录外部文本前可显式使用 `mask_sensitive_info`：

```rust,ignore
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
2. 用 `with_max_tokens` 控制响应规模
3. 配置适当的超时和重试策略
4. 对于批量操作，考虑使用并发调用

### Q: 如何处理并发请求？
A: 可以使用 Tokio 自带的 `JoinSet`，无需额外 stream/futures 依赖：

```rust,ignore
use tokio::task::JoinSet;
use zai_rs::{ZaiClient, model::*};

let client = ZaiClient::from_env()?;
let mut tasks = JoinSet::new();
for prompt in ["问题 1", "问题 2"] {
    let client = client.clone();
    tasks.spawn(async move {
        ChatCompletion::new(GLM4_5_flash {}, TextMessage::user(prompt))
            .send_via(&client)
            .await
    });
}

while let Some(result) = tasks.join_next().await {
    println!("{:?}", result??);
}
```

### Q: 连接池是如何工作的？
A: 一个 `ZaiClient` 及其克隆共享同一传输层和连接池。不要为每个请求重新构造
客户端。

---

## 功能特定问题

### Q: 实时 WebSocket 应该使用哪个模型？
A: 启用 `realtime` feature 后，使用 `GLM_realtime_flash` 或
`GLM_realtime_air`。Realtime 模型 trait 已密封，其他模型会在编译期被拒绝。
`GLM4_voice` 仅用于 HTTP 语音聊天，不能用于 Realtime WebSocket。旧 Realtime
marker 已在 0.6 删除：

```rust,ignore
use zai_rs::{model::GLM_realtime_flash, realtime::RealtimeClient};

let session = RealtimeClient::new(std::env::var("ZHIPU_API_KEY")?)
    .session(GLM_realtime_flash {})
    .build()
    .await?;
```

### Q: 如何使用工具调用（Function Calling）？
A: 定义工具并在请求中传递：

```rust,ignore
use zai_rs::{ZaiClient, model::*};

let client = ZaiClient::from_env()?;
let weather = Function::new(
    "get_weather",
    "Get weather information",
    serde_json::json!({
        "type": "object",
        "properties": {"city": {"type": "string"}},
        "required": ["city"]
    }),
);

let request = ChatCompletion::new(
    GLM4_5_flash {},
    TextMessage::user("What's the weather in Beijing?"),
)
.add_tool(Tools::Function { function: weather });
let response = request.send_via(&client).await?;
```

### Q: 如何上传文件？
A: 使用文件上传 API：

```rust,ignore
use zai_rs::{ZaiClient, file::*};

let client = ZaiClient::from_env()?;
let result = FileUploadRequest::new(FileUploadPurpose::Agent, "document.pdf")
    .with_content_type("application/pdf")
    .send_via(&client)
    .await?;
```

### Q: 如何使用知识库功能？
A: 对已有知识库上传文档，然后查询知识库详情：

```rust,ignore
use zai_rs::{ZaiClient, knowledge::*};

let client = ZaiClient::from_env()?;
let knowledge_id = "your-knowledge-id".to_string();

let upload = DocumentUploadRequest::new(knowledge_id.clone())
    .add_file_path("document.pdf")
    .send_via(&client)
    .await?;

let retrieve = KnowledgeGetRequest::new(knowledge_id)
    .send_via(&client)
    .await?;
```

### Q: 同一能力存在多个入口（图像生成 / OCR / 文件解析），应该选哪个？
A: 这些入口对应上游不同时期的 API，语义不同而非重复：图像生成默认用
同步的 `model::gen_image`，长耗时/批量用 `services::images` 异步任务；手写与
图片文字识别用 `model::ocr`，文档版式结构化解析用 `services::tools` 的
layout_parsing；小文件解析用 `file::parse_sync`，大文件/批量用
`tool::file_parser_create` + `tool::file_parser_result` 异步任务。完整对照见
[能力选择指引](CHOOSING_AN_API.md)。

---

## 故障排除

### Q: 连接超时怎么办？
A: 连接超时可以在 10 秒安全上限内收紧；单次请求默认 60 秒，大文件或慢速链路可
显式提高，最高 24 小时：

```rust,ignore
use std::time::Duration;
use zai_rs::client::HttpTransportConfig;

let config = HttpTransportConfig::builder()
    .connect_timeout(Duration::from_secs(5))?
    .request_timeout(Duration::from_secs(120))?
    .build();
```

### Q: 编译时出现 feature 错误怎么办？
A: 某些功能可能需要启用 feature：

```toml
[dependencies]
zai-rs = { version = "6.0.1", features = ["rmcp-kits"] }
```

### Q: 如何调试请求问题？
A: 可配置 `tracing` 查看 SDK 已提供的诊断事件；不要假设其中包含完整请求体或每次重试：

```rust,ignore
tracing_subscriber::fmt()
    .with_max_level(tracing::Level::DEBUG)
    .init();
```

---

## 其他

### Q: 是否支持异步/await？
A: 是的。所有网络请求都基于 Tokio 异步执行；请求构造、参数校验和部分本地辅助
方法仍是同步函数。

### Q: 如何贡献代码？
A: 欢迎贡献！请先提交 Issue 讨论您的想法，然后提交 Pull Request。确保代码通过 `cargo test` 和 `cargo clippy`。

### Q: 是否有示例代码？
A: 是的，请查看 [examples/](../examples/) 目录，包含各种使用场景的示例代码。
