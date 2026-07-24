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
`min(8s, 200ms × 2^n)`；`Retry-After` 的正整数秒或 IMF-fixdate 格式可能延长等待。

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

### 从目录安全加载工具

目录中的 JSON 只负责模型可见的名称、描述和参数 schema，不能声明本地缓存或重试
权限。旧的 `add_functions_from_dir_with_registry` 保持兼容，并固定使用
`CachePolicy::Never` / `RetryPolicy::Never`。若应用已经审计 handler 的副作用，可
在本地 registration 中显式声明：

```rust,ignore
use std::{collections::HashMap, sync::Arc};
use zai_rs::toolkits::{
    CachePolicy, RetryPolicy, ToolExecutionPolicy, ToolHandler,
    ToolRegistration, executor::ToolExecutor,
};

let handler: ToolHandler =
    Arc::new(|arguments| Box::pin(async move { Ok(arguments) }));
let registrations = HashMap::from([(
    "local_lookup".to_string(),
    ToolRegistration::new(handler).with_execution_policy(
        ToolExecutionPolicy::new(CachePolicy::Pure, RetryPolicy::Never),
    ),
)]);

let executor = ToolExecutor::builder().enable_cache().build();
let names = executor.add_functions_from_dir_with_registrations(
    "./tool-specs",
    &registrations,
    true,
)?;
```

`strict = true` 要求每份 JSON 都存在同名本地 registration；`false` 会跳过缺失项，
而没有对应文件的额外 registration 始终忽略。同一目录出现重复函数名，或目标
executor 已有同名工具时，整批拒绝且不留下部分注册。所有 JSON 与选中的 schema
也会在提交前完成解析/编译。JSON 中伪造的 `execution_policy` 字段始终被忽略；
可信策略只能来自进程内构造的 `ToolRegistration`。

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

    // 拉取式 chunk stream；慢消费者会自然施加背压。
    let mut chunks = FileContentRequest::new(file_id).stream_via(&client).await?;
    let mut received = 0usize;
    while let Some(chunk) = chunks.next().await {
        received += chunk?.len();
    }
    println!("文件字节数: {received}");

    // 也可直接写入新文件；已存在的目标绝不会被覆盖。
    let written = FileContentRequest::new(file_id)
        .send_to_via(&client, "download.bin")
        .await?;
    println!("已写入: {written}");

    Ok(())
}
```

文件内容总量上限为 128 MiB。瞬时错误只会在第一个 chunk 对调用者可见前重试；
中途断流会返回错误而不会重放已交付的字节。`send_to_via` 依赖目标文件系统支持
hard link，先同步文件内容再执行 no-clobber 发布；当前不承诺父目录项的掉电持久性。

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

使用有界并发，避免请求集合增长时同时占满连接、内存和上游配额：

```rust,ignore
use futures_util::{stream, StreamExt};
use zai_rs::{ZaiClient, model::*};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let queries = vec!["问题1", "问题2", "问题3"];
    let client = ZaiClient::from_env()?;

    let responses = stream::iter(queries.into_iter().map(|query| {
        let client = client.clone();
        async move {
            ChatCompletion::new(GLM4_5_flash {}, TextMessage::user(query))
                .send_via(&client)
                .await
        }
    }))
    .buffer_unordered(5);
    tokio::pin!(responses);

    while let Some(result) = responses.next().await {
        println!("{:?}", result?);
    }

    Ok(())
}
```

### 3. 合理设置超时

全局默认由 `HttpTransportConfig` 管理。单个慢文件、短交互或特殊幂等请求可以使用
带 `RequestOptions` 的轻量 client handle；它仍共享原 client 的连接池和凭据，选项
不会进入 JSON body：

```rust,ignore
use std::time::Duration;
use zai_rs::{
    ZaiClient,
    client::RequestOptions,
    file::FileContentRequest,
};

# async fn download(client: &ZaiClient) -> zai_rs::ZaiResult<()> {
let scoped = client.clone().with_request_options(
    RequestOptions::default()
        .with_attempt_timeout(Duration::from_secs(10 * 60))?
        .with_overall_timeout(Duration::from_secs(20 * 60))?
        .with_max_attempts(2)?,
);

let bytes = FileContentRequest::new("file-id")
    .send_via(&scoped)
    .await?;
# Ok(())
# }
```

优先级和作用范围：

| `RequestOptions` | 普通/文件请求 | SSE |
| --- | --- | --- |
| `attempt_timeout` | 每次尝试的绝对 deadline；文件流中也包含调用方暂停消费的时间 | 未单独设置时，作为 handshake 和 idle 默认值 |
| `overall_timeout` | 覆盖尝试、redirect 与 backoff | 只封顶 handshake，不限制已建立的长连接总寿命 |
| `sse_handshake_timeout` | 不适用 | 建立响应和校验错误响应的 deadline |
| `sse_idle_timeout` | 不适用 | 从最近一个响应 chunk 起算的绝对静默 deadline |
| `max_attempts` | 不超过全局 `max_attempts` | 忽略；SSE POST 永不自动重放 |
| `retry_override` | 显式声明幂等后，才允许 POST retry 和同源 307/308 重放 | 忽略 |

timeout override 可高于全局默认，但仍受 attempt/SSE 24 小时、overall 72 小时的绝对
上限约束；`max_attempts` 则始终以全局 transport 配置为上限。只有服务端具有可靠
幂等键或去重语义时，才可使用 `RetryOverride::AssumeIdempotent`。
SSE idle deadline 不会因调用方暂停 poll 而重新开始；恢复 poll 时，deadline 前已经
进入网络缓冲的 chunk 优先交付，否则已到期的静默会立即报错。当前实现保持 pull-based
背压，不为每条流启动无界后台 reader。

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
