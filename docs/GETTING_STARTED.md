# 快速入门指南

本指南将帮助您快速上手 `zai-rs` - Zhipu AI 的 Rust SDK。

## 前置要求

- Rust 1.88+（edition 2024）
- 智谱AI API Key（从 [智谱AI开放平台](https://open.bigmodel.cn/) 获取）

## 安装

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
zai-rs = "0.6"
```

## 配置

### 环境变量

SDK 默认从环境变量读取 API 密钥：

```bash
export ZHIPU_API_KEY="your-api-key-here"
```

### 高级配置

使用 `HttpTransportConfig` 配置统一传输层。连接超时保持 10 秒上限；单次请求默认
60 秒，可按慢速大文件传输需要显式提高（最高 24 小时）：

```rust,ignore
use zai_rs::client::{HttpTransportConfig, ZaiClient};
use std::time::Duration;

let transport = HttpTransportConfig::builder()
    .max_attempts(2)?
    .request_timeout(Duration::from_secs(30))?
    .build();
let client = ZaiClient::builder(std::env::var("ZHIPU_API_KEY")?)
    .transport(transport)
    .build()?;
```

若只有某一次调用需要不同 deadline，可在不新建连接池的情况下使用
`client.clone().with_request_options(RequestOptions::default()...)`。可分别设置
attempt、overall、SSE handshake 与 SSE idle；请求次数只能低于或等于全局
`max_attempts`，SSE 始终不会自动重放。完整矩阵见
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
use zai_rs::{ZaiClient, client::error::{ZaiError, ZaiResult}, model::*};

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
        Err(ZaiError::AuthError { code, message }) => {
            tracing::error!("认证错误 [{}]: {}", code, message);
        }
        Err(ZaiError::RateLimitError { code, message }) => {
            tracing::error!("速率限制 [{}]: {}", code, message);
        }
        Err(e) => {
            tracing::error!("发生错误: {}", e);
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
        Err(e) => tracing::error!("API 密钥格式错误: {}", e),
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
