# 快速入门指南

本指南将帮助您快速上手 `zai-rs` - Zhipu AI 的 Rust SDK。

## 前置要求

- Rust 1.70 或更高版本
- 智谱AI API Key（从 [智谱AI开放平台](https://open.bigmodel.cn/) 获取）

## 安装

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
zai-rs = "0.1"
```

## 配置

### 环境变量

SDK 默认从环境变量读取 API 密钥：

```bash
export ZHIPU_API_KEY="your-api-key-here"
```

### 高级配置

可以使用 `HttpClientConfig` 自定义 HTTP 客户端行为：

```rust,ignore
use zai_rs::client::http::{HttpClientConfig, RetryDelay};
use std::time::Duration;

// 使用自定义配置
let config = HttpClientConfig::builder()
    .max_retries(5)                    // 最多重试5次
    .timeout(Duration::from_secs(120))   // 超时时间120秒
    .retry_delay(
        RetryDelay::exponential(
            Duration::from_millis(100),  // 基础延迟100ms
            Duration::from_secs(10)       // 最大延迟10秒
        )
    )
    .logging(true)                     // 启用详细日志
    .mask_sensitive_data(true)           // 过滤敏感信息
    .build();
```

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
use zai_rs::model::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = GLM4_5_flash {};
    let messages = TextMessage::user("你好，请介绍一下你自己");
    let key = std::env::var("ZHIPU_API_KEY")?;
    
    let client = ChatCompletion::new(model, messages, key);
    let resp = client.post().await?;
    
    println!("{}", resp.choices().first().unwrap().message.content());
    Ok(())
}
```

### 2. 流式响应

对于实时响应，启用流式输出：

```rust,ignore
use zai_rs::model::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = GLM4_5_flash {};
    let messages = TextMessage::user("讲一个短故事");
    let key = std::env::var("ZHIPU_API_KEY")?;
    
    let mut client = ChatCompletion::new(model, messages, key)
        .enable_stream();
    
    let mut stream = client.sse_stream().await?;
    
    while let Some(chunk) = stream.next().await {
        if let Some(content) = chunk.choices().first().unwrap().delta.content() {
            print!("{}", content);
        }
    }
    
    Ok(())
}
```

### 3. 图像生成

```rust,ignore
use zai_rs::model::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = CogView {};
    let key = std::env::var("ZHIPU_API_KEY")?;
    
    let request = GenImageRequest::new(
        model,
        "一只可爱的猫咪",
        key
    );
    
    let resp = request.post().await?;
    println!("生成的图像URL: {:?}", resp.data().first().unwrap().url());
    
    Ok(())
}
```

### 4. 语音转文字

```rust,ignore
use zai_rs::model::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = GLM4_Audio {};
    let key = std::env::var("ZHIPU_API_KEY")?;
    let audio_file = std::fs::File::open("audio.mp3")?;
    
    let request = AudioToTextRequest::new(
        model,
        audio_file,
        key
    );
    
    let resp = request.post().await?;
    println!("识别结果: {}", resp.text());
    
    Ok(())
}
```

## 错误处理

SDK 提供了全面的错误类型：

```rust,ignore
use zai_rs::client::error::{ZaiError, ZaiResult};

async fn chat() -> ZaiResult<String> {
    let model = GLM4_5_flash {};
    let messages = TextMessage::user("Hello");
    let key = std::env::var("ZHIPU_API_KEY")?;
    
    let client = ChatCompletion::new(model, messages, key);
    let resp = client.post().await?;
    
    Ok(resp.choices().first().unwrap().message.content())
}

#[tokio::main]
async fn main() {
    match chat().await {
        Ok(content) => println!("Response: {}", content),
        Err(ZaiError::AuthError { code, message }) => {
            eprintln!("认证错误 [{}]: {}", code, message);
        }
        Err(ZaiError::RateLimitError { code, message }) => {
            eprintln!("速率限制 [{}]: {}", code, message);
        }
        Err(e) => {
            eprintln!("发生错误: {}", e);
        }
    }
}
```

## API 密钥验证

SDK 自动验证 API 密钥格式：

```rust,ignore
use zai_rs::client::error::validate_api_key;

fn main() {
    let api_key = "your-api-key.here";
    
    match validate_api_key(api_key) {
        Ok(()) => println!("API 密钥格式正确"),
        Err(e) => eprintln!("API 密钥格式错误: {}", e),
    }
}
```

## 高级配置

### 自定义参数

```rust,ignore
use zai_rs::model::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = GLM4_5_flash {};
    let messages = TextMessage::user("写一首诗");
    let key = std::env::var("ZHIPU_API_KEY")?;
    
    let mut client = ChatCompletion::new(model, messages, key);
    
    // 自定义生成参数
    client.body_mut().temperature = Some(0.7);
    client.body_mut().top_p = Some(0.9);
    client.body_mut().max_tokens = Some(1000);
    
    let resp = client.post().await?;
    println!("{}", resp.choices().first().unwrap().message.content());
    
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
