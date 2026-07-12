# 高级主题指南

本指南介绍 `zai-rs` SDK 的进阶功能和使用技巧。

## 目录

1. [重试机制](#重试机制)
2. [流式处理](#流式处理)
3. [工具调用](#工具调用)
4. [异步聊天](#异步聊天)
5. [实时 API](#实时-api)
6. [文件管理](#文件管理)
7. [知识库](#知识库)
8. [批量处理](#批量处理)
9. [性能优化](#性能优化)
10. [安全最佳实践](#安全最佳实践)

## 重试机制

### 概述

统一传输层只自动重试幂等方法（`GET`、`HEAD`、`OPTIONS`、`PUT`、
`DELETE`）。`POST` 和 `PATCH` 默认不会重放，以免重复产生服务端副作用。

### 重试策略

重试采用 full jitter。第 `n` 次重试的随机等待上限为
`min(8s, 200ms × 2^n)`；正整数秒格式的 `Retry-After` 可能延长等待。

### 哪些错误会重试

| 错误类型 | 重试 | 说明 |
|----------|------|------|
| 408/425/429/500/502/503/504 | 条件重试 | 仅限幂等请求，且业务码未将其排除 |
| 网络错误 | 条件重试 | 仅限幂等请求和剩余尝试次数 |
| POST/PATCH | ❌ | 默认不重放 |
| 其他 4xx/5xx | ❌ | 不在固定重试状态集合中 |
| 认证错误 | ❌ | 需要修正 API 密钥 |
| 配额、校验、内容策略业务码 | ❌ | 业务码优先于 HTTP 状态 |

### 重试限制

默认配置：
- `max_attempts = 3`，包含首次请求，因此最多重试两次
- full-jitter 基础上限 `200ms`，指数增长并封顶 `8s`

所有重试都失败后，返回最后一次错误。

## 流式处理

### 类型安全的 SSE 聊天流

`ChatCompletion::enable_stream()` 把请求切换到流式类型状态，随后
`stream_via(&client)` 返回 `ChatStreamResponse` 流。API 密钥不会离开
`ZaiClient`，内容类型、超时与大小限制也由统一传输层处理：

```rust,ignore
use zai_rs::{ZaiClient, model::*};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let mut stream = ChatCompletion::new(
        GLM4_5_flash {},
        TextMessage::user("写一个科幻故事"),
    )
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
    Ok(())
}
```

## 工具调用

### 概述

工具调用允许 LLM 与外部系统和 API 交互。SDK 提供了完整的工具调用支持。

### 定义工具

```rust,ignore
use zai_rs::model::tools::{Function, Tools};
use serde_json::json;

fn get_weather() -> Tools {
    Tools::Function {
        function: Function::new(
            "get_weather",
            "获取指定城市的天气信息",
            json!({
                "type": "object",
                "properties": {"city": {"type": "string"}},
                "required": ["city"]
            }),
        ),
    }
}
```

### 使用工具

```rust,ignore
use zai_rs::{ZaiClient, model::*};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let request = ChatCompletion::new(
        GLM4_5_flash {},
        TextMessage::system("你是一个有用的助手。"),
    )
    .add_message(TextMessage::user("北京今天的天气如何？"))
    .add_tool(get_weather());
    let resp = request.send_via(&client).await?;

    // 检查是否有工具调用
    if let Some(tool_calls) = resp
        .choices()
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.message())
        .and_then(|message| message.tool_calls())
    {
        for call in tool_calls {
            if let Some(function) = call.function() {
                println!("调用工具: {}", function.name());
                println!("参数: {}", function.arguments());
            }
        }
    }

    Ok(())
}
```

### 使用 Toolkits

`model::Tools` 描述发送给模型的工具协议；本地 `toolkits` 模块提供独立的工具
注册和执行框架，`toolkits` feature 额外启用 JSON-Schema 参数校验。两者不是同一
种类型。完整闭环请参考
`examples/function_call_with_toolkits.rs`，不要把 `model::Tools` 直接注册到
`ToolExecutor`。

## 异步聊天

### 概述

异步聊天 API 允许提交长时间运行的任务，稍后轮询获取结果。

### 提交异步任务

```rust,ignore
use zai_rs::{ZaiClient, model::*};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let request = AsyncChatCompletion::new(
        GLM4_5_flash {},
        TextMessage::user("分析这段长文本..."),
    );
    let resp = request.send_via(&client).await?;

    let task_id = resp.id().ok_or("async response omitted task id")?;
    println!("任务ID: {}", task_id);

    Ok(())
}
```

### 获取异步结果

```rust,ignore
use std::time::Duration;
use zai_rs::{ZaiClient, model::*};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let task_id = "your-task-id";
    let request = AsyncTaskGetRequest::new(task_id);

    // 最多轮询 60 次，避免服务端异常时永久挂起。
    for _ in 0..60 {
        let resp = request.send_via(&client).await?;

        match resp {
            AsyncTaskResult::Chat(result) => {
                println!("任务完成");
                if let Some(content) = result
                    .choices()
                    .and_then(|choices| choices.first())
                    .and_then(|choice| choice.message())
                    .and_then(|message| message.content_str())
                {
                    println!("结果: {}", content);
                }
                break;
            }
            AsyncTaskResult::State(state) if state.is_processing() => {
                println!("处理中...");
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            AsyncTaskResult::State(state) if state.is_failed() => {
                tracing::error!("任务失败");
                break;
            }
            AsyncTaskResult::State(_) => {
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            AsyncTaskResult::Video(_) | AsyncTaskResult::Image(_) => {
                tracing::error!("未知状态");
                break;
            }
        }
    }
    // 生产代码应把“超过轮询上限”转换为自己的超时错误。

    Ok(())
}
```

## 实时 API

实时 WebSocket 需启用 `realtime` feature。当前协议只接受
`GLM_realtime_flash`（`glm-realtime-flash`）和
`GLM_realtime_air`（`glm-realtime-air`）：

```rust,ignore
use zai_rs::{
    model::GLM_realtime_flash,
    realtime::{RealtimeClient, TurnDetectionType},
};

let key = std::env::var("ZHIPU_API_KEY")?;
let session = RealtimeClient::new(key)
    .session(GLM_realtime_flash {})
    .turn_detection(TurnDetectionType::ServerVad)
    .build()
    .await?;
session.send_text("你好").await?;
session.create_response().await?;
```

首次调用 `events()` / `audio_stream()` 会收到会话建立后已经缓冲的事件；两个流的
元素都是 `ZaiResult`。消费速度不足导致丢帧或后台会话异常时，流会返回错误并
终止，调用方不能把不完整 PCM 当作成功结果。音频元素为 `RealtimeAudioChunk`，
其 `data` 是 24 kHz、单声道、16 位小端 PCM，关联 ID 保留在同一结构中。

Realtime 模型 trait 已密封，其他模型会在编译期被拒绝。`GLM4_voice` 仅用于
HTTP 语音聊天，不能用于 Realtime WebSocket；无可用操作能力的旧 Realtime
marker 已在 0.6 删除。

## 文件管理

### 上传文件

```rust,ignore
use zai_rs::{ZaiClient, file::*};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let upload = FileUploadRequest::new(FileUploadPurpose::Batch, "document.pdf")
        .send_via(&client)
        .await?;
    println!("文件ID: {:?}", upload.id);

    Ok(())
}
```

### 列出文件

```rust,ignore
use zai_rs::{ZaiClient, file::*};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let list = FileListRequest::new(FileListPurpose::Batch)
        .send_via(&client)
        .await?;

    for file in list.data.as_deref().unwrap_or_default() {
        println!("{:?}: {:?}", file.id, file.filename);
    }

    Ok(())
}
```

### 获取文件内容

```rust,ignore
use zai_rs::{ZaiClient, file::*};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let file_id = "file-abc123";

    let content = FileContentRequest::new(file_id).send_via(&client).await?;
    println!("文件字节数: {}", content.len());

    Ok(())
}
```

### 删除文件

```rust,ignore
use zai_rs::{ZaiClient, file::*};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let file_id = "file-abc123";

    FileDeleteRequest::new(file_id).send_via(&client).await?;

    println!("文件已删除");

    Ok(())
}
```

## 知识库

### 创建知识库

```rust,ignore
use zai_rs::{ZaiClient, knowledge::*};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let resp = KnowledgeCreateRequest::new(EmbeddingId::Embedding3New, "我的知识库")
        .with_description("描述")
        .send_via(&client)
        .await?;
    println!(
        "知识库ID: {:?}",
        resp.data.as_ref().and_then(|data| data.id.as_deref())
    );

    Ok(())
}
```

### 上传文档

```rust,ignore
use zai_rs::{ZaiClient, knowledge::*};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let kb_id = "kb-abc123";
    let resp = DocumentUploadRequest::new(kb_id)
        .add_file_path("document.pdf")
        .send_via(&client)
        .await?;
    println!("上传结果: {resp:#?}");

    Ok(())
}
```

### 查询知识库

```rust,ignore
use zai_rs::{ZaiClient, knowledge::*};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let kb_id = "kb-abc123";
    let query = "什么是机器学习？";

    let resp = KnowledgeSearchRequest::new(kb_id, query)
        .send_via(&client)
        .await?;
    println!("相关内容: {:?}", resp);

    Ok(())
}
```

## 批量处理

### 创建批量任务

```rust,ignore
use zai_rs::{ZaiClient, batches::*, file::*};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;

    // 1. 上传包含批量请求的文件
    let file = FileUploadRequest::new(FileUploadPurpose::Batch, "requests.jsonl")
        .send_via(&client)
        .await?;
    let file_id = file.id.ok_or("upload response omitted file id")?;

    // 2. 创建批量任务
    let resp = BatchCreateRequest::new(file_id, BatchEndpoint::ChatCompletions)
        .send_via(&client)
        .await?;
    println!("批量任务ID: {:?}", resp.id);

    Ok(())
}
```

### 检查批量任务状态

```rust,ignore
use zai_rs::{ZaiClient, batches::*};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let batch_id = "batch-abc123";

    let batch = BatchGetRequest::new(batch_id)
        .send_via(&client)
        .await?;
    println!("状态: {:?}", batch.status);
    println!("请求计数: {:?}", batch.request_counts);

    Ok(())
}
```

### 取消批量任务

```rust,ignore
use zai_rs::{ZaiClient, batches::*};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let batch_id = "batch-abc123";

    BatchCancelRequest::new(batch_id)
        .send_via(&client)
        .await?;

    println!("批量任务已取消");

    Ok(())
}
```

## 性能优化

### 1. 连接复用

每个 `ZaiClient` 持有并复用一个 HTTP 连接池：

```rust,ignore
use zai_rs::{ZaiClient, model::*};

let client = ZaiClient::from_env()?;

// ✅ 所有请求共享一个 ZaiClient 连接池
for i in 0..10 {
    let request = ChatCompletion::new(
        GLM4_5_flash {},
        TextMessage::user(format!("问题 {i}")),
    );
    let resp = request.send_via(&client).await?;
}
```

### 2. 并发请求

使用 Tokio 进行并发处理：

```rust,ignore
use tokio::task::JoinSet;
use zai_rs::{ZaiClient, model::*};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let queries = vec!["问题1", "问题2", "问题3"];
    let client = ZaiClient::from_env()?;

    let mut tasks = JoinSet::new();
    for query in queries {
        let client = client.clone();
        tasks.spawn(async move {
            ChatCompletion::new(GLM4_5_flash {}, TextMessage::user(query))
                .send_via(&client)
                .await
        });
    }

    while let Some(result) = tasks.join_next().await {
        println!("{:?}", result??);
    }

    Ok(())
}
```

### 3. 合理设置超时

```rust,ignore
// 通过 ZaiClient::builder(...).transport(HttpTransportConfig) 收紧统一传输层超时。
```

## 安全最佳实践

### 1. 保护 API 密钥

```rust,ignore
// ✅ 从环境变量读取
let key = std::env::var("ZHIPU_API_KEY")?;

// ❌ 不要硬编码
let key = "sk-abc123.xyz";
```

### 2. 不记录敏感信息

```rust,ignore
use zai_rs::client::error::mask_sensitive_info;

fn log_external_message(message: &str) {
    // 仅在必须记录外部文本时做兜底清理；不要先把 API key 拼进日志。
    tracing::info!(message = %mask_sensitive_info(message));
}
```

### 3. 验证 API 密钥

```rust,ignore
use zai_rs::client::error::validate_api_key;

let key = std::env::var("ZHIPU_API_KEY")?;
validate_api_key(&key)?;
```

## 相关资源

- [快速入门指南](GETTING_STARTED.md)
- [错误处理指南](ERROR_HANDLING.md)
- [智谱AI API 文档](https://docs.bigmodel.cn/)
- [示例代码](../examples/)
