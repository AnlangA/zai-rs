# 快速入门指南

本指南将帮助您快速上手 `zai-rs` - Zhipu AI 的 Rust SDK。

## 前置要求

- Rust 1.88+（edition 2024）
- 智谱AI API Key（从 [智谱AI开放平台](https://open.bigmodel.cn/) 获取）

## 安装

当前文档描述的 API 尚未发布到 crates.io。`Cargo.toml` 中的 `6.1.0` 是
未发布候选版（取代同样未发布的 `6.0.1`）；`6.0.1` 的两次 tag workflow
分别因当时 tag 不是 annotated tag，以及 crates.io 缺少 `AnlangA/zai-rs`
Trusted Publisher 配置而失败。使用当前仓库 API 时，应将依赖绑定到已审计
commit：

```toml
[dependencies]
zai-rs = { git = "https://github.com/AnlangA/zai-rs", rev = "<audited-commit>" }
```

经 `cargo search` / `cargo info` 验证的 crates.io 最新版为 `0.6.0`：

```toml
[dependencies]
zai-rs = "0.6.0"
```

`0.6.0` 是 legacy registry 版本，其 API 和行为与本指南描述的当前工作树存在
差异。工作树已偏离 `v6.0.1` tag，正式发布使用高于 `6.0.1` 的 `6.1.0`
候选版和新 tag，且需先完成 Trusted Publisher 配置。

## 配置

### 环境变量

SDK 默认从环境变量读取 API 密钥：

```bash
export ZHIPU_API_KEY="your-api-key-here"
```

### 高级配置

使用 `HttpTransportConfig` 配置统一传输层，并用独立的
`HttpConcurrencyConfig` 配置逻辑请求准入。后者默认允许 64 个并发操作，permit
覆盖 buffered 请求的全部 retry/backoff；SSE/文件 stream 则持有到结束、调用方
Drop 或安全 lease 到期。SSE consumer 的 configured base 默认 5 分钟；lease 只在
底层 raw-stream poll 实际取得 chunk 时续期，typed decoder 从已缓冲 raw chunk 产出
item 不保证续期。实际间隔按 `base = min(scoped, global)`，再按
`effective = max(base, sse_idle + 1s)` 计算。base setter 的范围是 `1ns..=24h`，但
idle floor 可使 scoped override 不生效，并使 effective 最大达到 24 小时加 1 秒。
文件流复用其 absolute overall deadline。lease 到期会整体回收 response body 与 permit，
而不是让仍占 socket 的 body 逃出并发预算。超出预算的请求默认最多排队 30 秒。
连接超时保持 10 秒上限；单次请求默认 60 秒，可按慢速大文件传输需要显式提高
（最高 24 小时）：

```rust,ignore
use zai_rs::client::{HttpConcurrencyConfig, HttpTransportConfig, ZaiClient};
use std::time::Duration;

let transport = HttpTransportConfig::builder()
    .max_attempts(2)?
    .request_timeout(Duration::from_secs(30))?
    .build();
let concurrency = HttpConcurrencyConfig::default()
    .with_max_in_flight(32)?
    .with_queue_timeout(Duration::from_secs(5))?
    .with_stream_consumer_timeout(Duration::from_secs(120))?;
let client = ZaiClient::builder(std::env::var("ZHIPU_API_KEY")?)
    .transport(transport)
    .concurrency(concurrency)
    .build()?;
```

若只有某一次调用需要不同 deadline，可在不新建连接池的情况下使用
`client.clone().with_request_options(RequestOptions::default()...)`。可分别设置
admission queue、attempt、overall、SSE handshake、SSE idle 与 SSE consumer lease；
per-request queue timeout 只能缩短全局值，设为零可在没有空闲 permit 时立即失败。
consumer override 只能降低 configured base；若 `sse_idle + 1s` 更大，则不会改变
effective lease。该 floor 保证 consumer deadline 始终严格晚于 network idle，因此
“服务端无数据”和“底层 raw stream 未被推进”会分别报告 `SseIdle` 与
`StreamConsumer`。
文件流不使用 SSE consumer 配置，始终由 absolute overall deadline 回收。请求次数只能
低于或等于全局 `max_attempts`，SSE 始终不会自动重放。完整矩阵见
[高级主题](ADVANCED_TOPICS.md#3-合理设置超时)。

### 日志配置

使用 `tracing` 进行结构化日志记录：

```rust,ignore
use tracing_subscriber;

fn init_logging() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("zai_rs=debug")
        )
        .init();
}
```

## 基础用法

### 1. 聊天补全

最简单的文本聊天：

```rust,ignore
use zai_rs::{client::ZaiClient, model::*};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = GLM4_5_flash {};
    let messages = TextMessage::user("你好，请介绍一下你自己");
    let client = ZaiClient::from_env()?;
    let request = ChatCompletion::new(model, messages);
    let resp = request.send_via(&client).await?;

    if let Some(text) = resp
        .choices()
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.message())
        .and_then(|message| message.content_str())
    {
        println!("{text}");
    }
    Ok(())
}
```

### 2. 流式聊天响应

调用 `enable_stream()` 进入流式类型状态，再用 `stream_via(&client)` 获取
强类型 SSE 流。鉴权和响应限制始终由 `ZaiClient` 管理：

```rust,ignore
use zai_rs::{client::ZaiClient, model::*};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = GLM4_5_flash {};
    let messages = TextMessage::user("讲一个短故事");
    let client = ZaiClient::from_env()?;
    let mut stream = ChatCompletion::new(model, messages)
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

    Ok(())
}
```

### 3. 图像生成

```rust,ignore
use zai_rs::model::gen_image::*;
use zai_rs::ZaiClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let request = ImageGenRequest::new(GlmImage {})
        .with_prompt("一只可爱的猫咪")
        .with_size(ImageSize::Size1280x1280)
        .with_quality(ImageQuality::Hd);
    let resp: ImageResponse = request.send_via(&client).await?;
    println!("生成的图像: {:#?}", resp);

    Ok(())
}
```

### 4. 语音转文字

```rust,ignore
use zai_rs::model::audio_to_text::{AudioToTextRequest, AudioToTextResponse, GlmAsr};
use zai_rs::ZaiClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let request = AudioToTextRequest::new(GlmAsr {}).with_file_path("audio.mp3");
    let resp: AudioToTextResponse = request.send_via(&client).await?;
    println!("识别结果: {:#?}", resp);

    Ok(())
}
```

## 错误处理

SDK 提供了全面的错误类型：

```rust,ignore
use zai_rs::{ZaiClient, client::error::ZaiResult, model::*};

async fn chat() -> ZaiResult<String> {
    let client = ZaiClient::from_env()?;
    let response = ChatCompletion::new(GLM4_5_flash {}, TextMessage::user("Hello"))
        .send_via(&client)
        .await?;

    Ok(response
        .choices()
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.message())
        .and_then(|message| message.content_str())
        .unwrap_or_default()
        .to_owned())
}

#[tokio::main]
async fn main() {
    match chat().await {
        Ok(content) => println!("Response: {}", content),
        Err(error) if error.is_auth_error() => {
            tracing::error!(category = ?error.category(), "认证错误");
        }
        Err(error) if error.is_rate_limit() => {
            tracing::warn!(category = ?error.category(), "请求受到速率限制");
        }
        Err(error) => {
            tracing::error!(
                category = ?error.category(),
                retryable = error.is_retryable(),
                "API 请求失败",
            );
        }
    }
}
```

## API 密钥验证

`ZaiClient` 构建时会拒绝空值、空白字符和不能安全写入认证 header 的 API key。
如需额外验证常见的 `id.secret` 形态，请显式调用：

```rust,ignore
use zai_rs::client::error::validate_api_key;

fn main() {
    let api_key = "example-id.example-secret-value";

    match validate_api_key(api_key) {
        Ok(()) => println!("API 密钥格式正确"),
        Err(_) => tracing::error!("API 密钥格式不符合要求"),
    }
}
```

## 高级配置

### 自定义参数

```rust,ignore
use zai_rs::{ZaiClient, model::*};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let request = ChatCompletion::new(GLM4_5_flash {}, TextMessage::user("写一首诗"))
        .with_temperature(0.7)
        .with_top_p(0.9)
        .with_max_tokens(1000);

    let resp = request.send_via(&client).await?;
    println!("{resp:#?}");

    Ok(())
}
```

### 日志敏感信息过滤

SDK 提供了敏感信息过滤功能，用于安全日志：

```rust,ignore
use zai_rs::client::error::mask_sensitive_info;

fn main() {
    let log_text = "API key: abc123.abcdefghijklmnopqrstuvwxyz12345, password: secret";
    let filtered = mask_sensitive_info(log_text);

    // 输出: API key: [FILTERED], password: [FILTERED]
    println!("{}", filtered);
}
```

## 更多示例

- 查看 [examples](../examples/) 目录了解更多用法示例
- 参考 [API 文档](../src/) 获取完整 API 参考

## 支持与帮助

- [智谱AI API 文档](https://docs.bigmodel.cn/)
- [GitHub Issues](https://github.com/AnlangA/zai-rs/issues)
