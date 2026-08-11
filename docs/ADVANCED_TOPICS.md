# 高级主题指南

本指南介绍 `zai-rs` SDK 的进阶功能和使用技巧。

## 目录

1. [重试机制](#重试机制)
2. [流式处理](#流式处理)
3. [工具调用](#工具调用)
4. [异步聊天](#异步聊天)
5. [实时 API](#实时-api)
6. [分页原语](#分页原语)
7. [文件管理](#文件管理)
8. [知识库](#知识库)
9. [批量处理](#批量处理)
10. [性能优化](#性能优化)
11. [安全最佳实践](#安全最佳实践)

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
use std::time::Duration;
use zai_rs::{
    model::GLM_realtime_flash,
    realtime::{RealtimeClient, RealtimeTransportConfig, TurnDetectionType},
};

# async fn run(key: String) -> zai_rs::ZaiResult<()> {
let transport_config = RealtimeTransportConfig::builder()
    // 内建会话的全部建连尝试与等待共享该总预算。
    .connect_timeout(Duration::from_secs(8))
    .inbound_idle_timeout(Duration::from_secs(120))
    .try_build()?;
let session = RealtimeClient::new(key)
    .with_transport_config(transport_config)
    .session(GLM_realtime_flash {})
    .turn_detection(TurnDetectionType::ServerVad)
    .build()
    .await?;
session.send_text("你好").await?;
session.create_response().await?;
# session.close().await?;
# Ok(())
# }
```

公开配置的规范路径是
`zai_rs::realtime::RealtimeTransportConfig`。`Default` 沿用配置公开前的主要
timeout/capacity 数值，但不是逐行为复刻：出站调用新增默认 30 秒的 admission 总
deadline，单个 data frame 的默认 stall guard 也从 10 秒收紧到 5 秒。这两项是可观察的
有界加固；内建会话还新增默认最多 3 次的安全首次连接恢复行为。
12 个主配置项包括：

- `connect_timeout`（默认 10 秒）和 `max_connect_attempts`（默认 3，范围
  `1..=3`，`1` 表示禁用连接重试）；
- `write_timeout`（30 秒）、
  `pong_timeout`（10 秒）、`close_timeout`（5 秒）、
  `inbound_idle_timeout`（90 秒）和 `outbound_queue_timeout`（30 秒）；
- `outbound_queue_capacity`、`writer_queue_capacity`、
  `event_buffer_capacity`、`audio_buffer_capacity`（均默认 8）；
- `max_frame_bytes`（默认 2 MiB）。

各项的精确合法区间及交叉约束以 `RealtimeTransportConfig` API 文档中的表为准；
builder 的 setter 与顺序无关，由 `try_build()` 一次性校验。例如 Pong deadline 不得
大于完整消息 write deadline，inbound idle 必须大于派生的普通 send deadline。

`RealtimeClient::with_transport_config` 设置后续 session builder 的默认策略；调用
`session(...)` 时会快照该值，随后 builder 上的 `with_transport_config` 会完整替换这份
快照，只影响该会话。建成后可通过 `RealtimeSession::transport_config()` 查看最终策略。

| 策略 | `SessionBuilder::build` 内建会话 | 直接 `connect_with_config` | `build_with_transport` 注入会话 |
|------|----------------------------------|------------------------------|----------------------------------|
| 连接获取 | 最多 `max_connect_attempts` 次；尝试、退避和 `Retry-After` 共享 `connect_timeout` 总预算 | 始终单次，`connect_timeout` 只约束该次尝试；attempt 数仅保留供 getter 检查 | 不适用；transport 已连接并由应用认证 |
| Pong、frame、writer queue | SDK 执行 | SDK 执行 | 不适用；由注入实现自行负责 |
| socket write / close | SDK 执行，并用于派生 session guard | SDK 执行 | 不控制注入 socket；SDK 只执行派生的 initial/send/close 外层 guard |
| inbound idle、outbound admission/queue、event/audio buffer | SDK 执行 | 不执行，仅在 config getter 中保留 | SDK 执行 |

因此只有 `SessionBuilder` 创建的内建 Tungstenite 会话消费全部 12 项；直接连接只消费
wire 侧 connect timeout、write/Pong/close/writer/frame 设置，不执行
`max_connect_attempts`，但会完整保留配置供 `TungsteniteTransport::transport_config()`
检查。配置对象不会传给注入 transport。

非零 `outbound_queue_timeout` 是贯穿单并发
prepare、WAV/base64/JSON 构造、精确 byte-budget 获取和 command-channel 准入的总
deadline，并在各阶段边界复查；设为零时所有可竞争准入都 fail-fast，不等待空位。
媒体构造会把 stack WAV header+PCM、raw PCM 或 JPEG 直接 base64 写入精确容量的最终
JSON；public `ClientEvent` wire 不变，同时不再保留 payload-sized base64 中间 String。
调大消息数量 capacity 也不能突破不可配置的安全上限：序列化消息和内建 session
端到端 byte budget 最大 8 MiB，直接使用 `TungsteniteTransport` 时 writer 另有 8 MiB
budget，单个原始 audio/video 最大 4 MiB，同时只允许一个 outbound preparation。
内建 session 的端到端消息数取 outbound/writer capacity 的较小值；一条已接受命令的
byte/count permit 会一直跟随到 socket writer 完成，因此不会在 API 返回成功后再因第二层
准入已满而终止会话。这些上限不能通过配置抬高。

内建 confirmed write 与普通 transport send guard 为 `write_timeout + 1s`；仅注入路径
再以 `write_timeout + 2s` 保护 initial update。writer join 为
`close_timeout + 1s`，session/injected close guard 为 `close_timeout + 2s`。完整消息仍受
`write_timeout` 约束，但单个 data frame 的 stall guard 不再直接共用 Pong deadline，
而是 `min(5s, pong_timeout / 2)`；Pong 保留包含排队时间的独立绝对 deadline。

内建 writer 在 RFC control 与应用 data 之间使用显式公平轮转。通常先处理 Pong；每次
成功写入 control 后会切换到 data 偏好并让出一次调度机会，因此即使对端在每个 Pong 后
立刻反馈下一次 Ping，已排队或随后到达的应用消息也会继续推进。完成一条应用消息后则
重新偏好 control，持续 data backlog 不会饿死 Pong。shutdown 在两种偏好下始终最高，
control 只会插入应用消息的 frame 边界，应用消息本身仍按 FIFO 完整写出、互不穿插。

旧的 `RealtimeClient::new(...).session(...).build()` 继续使用 `Default`，因此内建
builder 默认最多尝试连接 3 次；`TungsteniteTransport::connect(...)` 和显式
`connect_with_config(...)` 仍各自只执行一次直接连接。内建 builder 只重试可恢复的网络
或握手失败，且严格限于发送首个 `session.update` 之前；full-jitter 退避、有效
`Retry-After` 和每次连接尝试共享同一 `connect_timeout` 绝对总预算。JWT 模式会在每次
尝试前重新签发凭证；一旦连接成功并开始发送 `session.update`，写入结果可能不明确，SDK
不会重放。
只实现 `send` / `recv` / `close` 的既有 `RealtimeTransport` 仍保持源码兼容；注入路径
接收的是已连接 transport，不参与上述连接重试。

首次调用 `events()` / `audio_stream()` 会收到会话建立后已经缓冲的事件；两个流的
元素都是 `ZaiResult`。消费速度不足导致丢帧或后台会话异常时，流会返回错误并
终止，调用方不能把不完整 PCM 当作成功结果。音频元素为 `RealtimeAudioChunk`，
其 `data` 是 24 kHz、单声道、16 位小端 PCM，关联 ID 保留在同一结构中。

Realtime 模型 trait 已密封，其他模型会在编译期被拒绝。`GLM4_voice` 仅用于
HTTP 语音聊天，不能用于 Realtime WebSocket；无可用操作能力的旧 Realtime
marker 已在 0.6 删除。

## 分页原语

`CursorPagination` 和 `PagePagination` 提供可复用、受检的分页值；通过请求上的
`try_with_pagination` 映射到具体 endpoint：

```rust,ignore
use zai_rs::{
    batches::BatchListRequest,
    file::{FileListPurpose, FileListRequest},
    pagination::{CursorPagination, PagePagination},
    services::assistants::{AssistantConversationListRequest, AssistantId},
};

fn build_paginated_requests() -> zai_rs::ZaiResult<()> {
    let cursor = CursorPagination::new()
        .try_with_after("opaque-cursor")?
        .try_with_limit(20)?;
    let batches = BatchListRequest::new().try_with_pagination(cursor.clone())?;
    let files = FileListRequest::new(FileListPurpose::Batch)
        .try_with_pagination(cursor)?;

    let page = PagePagination::try_new(2, 50)?;
    let conversations = AssistantConversationListRequest::new(AssistantId::ChatGlm)
        .try_with_pagination(page)?;

    let _ = (batches, files, conversations);
    Ok(())
}
```

两个类型都会拒绝零值，cursor 还会拒绝纯空白值并在 `Debug` 中脱敏；opaque cursor 的
原始内容会保留到 URL 编码边界。通用类型只校验共同下限，附着到请求时还会执行 endpoint
上限：文件列表的 `limit` 与 assistant conversation 的 `page_size` 均为 `1..=100`，
batch、knowledge 和 document list 当前不另设 SDK 上限（provider 仍可能执行服务端约束）。

分页类型刻意不实现 `Serialize`。不同 endpoint 会把同一语义映射为 `after` / `limit`、
`page` / `page_size` 或 `page` / `size`，并可能要求其他查询或 body 字段；应始终通过具体
请求的 `try_with_pagination` 生成 wire shape，而不是直接序列化分页值。

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
中途断流会返回错误而不会重放已交付的字节。`send_to_via` 先同步完整文件，再以
hard link 或仅在其明确不受支持时使用的 no-clobber fallback 发布，已有目标绝不会
被覆盖。Unix 会在创建父目录前记录 lexical parent chain，并在发布后从目标的直接父目录
开始，按 deepest-first 顺序同步每个新建祖先直到首个预存 anchor。在 namespace 稳定时，
成功返回覆盖文件内容、目标目录项及本次创建的每级目录项。该协议不会 canonicalize 或
固定 symlink；其他进程在下载期间替换 path component 不在保证范围内。任一目录同步在
发布后失败都会返回 `SDK_IO`，但完整目标已经存在且不会回滚，即
published-but-durability-unconfirmed；调用方应先检查并协调目标，不能盲目重试。
Windows/其他 non-Unix 平台在 stable Rust 下没有可移植的目录同步保证，因此仍只承诺
发布前的文件内容同步，不承诺目录项掉电存活。

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
