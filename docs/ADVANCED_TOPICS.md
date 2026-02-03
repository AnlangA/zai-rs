# 高级主题指南

本指南介绍 `zai-rs` SDK 的进阶功能和使用技巧。

## 目录

1. [重试机制](#重试机制)
2. [配置选项](#配置选项)
3. [日志和追踪](#日志和追踪)
4. [流式处理](#流式处理)
5. [工具调用](#工具调用)
6. [GLM-4.5 思考模式](#glm-45-思考模式)
7. [OCR 手写识别](#ocr-手写识别)
8. [Agent API](#agent-api)
9. [实时 API](#实时-api)
10. [异步聊天](#异步聊天)
11. [文件管理](#文件管理)
12. [知识库](#知识库)
13. [批量处理](#批量处理)

## 重试机制

### 概述

SDK 内置了智能重试机制，自动处理临时性故障，提高请求成功率。

### 重试策略

SDK 使用指数退避算法配合随机抖动：

```rust
// 重试延迟计算
delay = base * 2^attempt
delay_with_jitter = delay + random(0, delay/4)

// 示例（base=500ms, max=5s）
attempt 0: 500ms  + 0-125ms
attempt 1: 1000ms + 0-250ms
attempt 2: 2000ms + 0-500ms
attempt 3: 4000ms + 0-1000ms (capped at 5s)
```

### 哪些错误会重试

| 错误类型 | 重试 | 说明 |
|----------|------|------|
| 5xx 服务器错误 | ✅ | 临时性服务器故障 |
| 速率限制错误（1301） | ✅ | 短期内稍后重试 |
| 网络错误 | ✅ | 连接问题 |
| 4xx 客户端错误 | ❌ | 请求参数错误 |
| 认证错误 | ❌ | 需要修正 API 密钥 |

### 重试日志

SDK 会记录重试信息：

```
WARN Request failed (attempt 1/4), retrying after 512ms: HTTP[503]: Service Unavailable
WARN Request failed (attempt 2/4), retrying after 1.2s: HTTP[503]: Service Unavailable
```

### 重试限制

默认配置：
- 最大重试次数：3
- 基础延迟：500ms
- 最大延迟：5s

所有重试都失败后，返回最后一次错误。

## 流式处理

### SSE 流式响应

对于实时响应需求，使用 Server-Sent Events (SSE) 流：

```rust,ignore
use zai_rs::model::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = GLM4_5_flash {};
    let messages = TextMessage::user("写一个科幻故事");
    let key = std::env::var("ZHIPU_API_KEY")?;
    
    // 启用流式输出
    let mut client = ChatCompletion::new(model, messages, key)
        .enable_stream();
    
    // 获取 SSE 流
    let mut stream = client.sse_stream().await?;
    
    // 处理流式数据
    while let Some(chunk) = stream.next().await {
        if let Some(delta) = chunk.choices().first().unwrap().delta.content() {
            print!("{}", delta);
            std::io::stdout().flush()?;
        }
    }
    
    println!("\n--- 完成 ---");
    Ok(())
}
```

### 流式响应处理技巧

#### 1. 累积完整响应

```rust,ignore
let mut full_response = String::new();
let mut stream = client.sse_stream().await?;

while let Some(chunk) = stream.next().await {
    if let Some(delta) = chunk.choices().first().unwrap().delta.content() {
        full_response.push_str(delta);
        print!("{}", delta);
    }
}

println!("\n完整响应: {}", full_response);
```

#### 2. 处理流式错误

```rust,ignore
let mut stream = client.sse_stream().await?;

loop {
    match stream.next().await {
        Some(Ok(chunk)) => {
            // 处理正常的流式数据
        }
        Some(Err(e)) => {
            eprintln!("流式错误: {}", e);
            // 可以选择继续或终止
            break;
        }
        None => {
            // 流结束
            println!("流式响应完成");
            break;
        }
    }
}
```

## 工具调用

### 概述

工具调用允许 LLM 与外部系统和 API 交互。SDK 提供了完整的工具调用支持。

### 定义工具

```rust,ignore
use zai_rs::model::tools::*;
use serde_json::{json, Value};

fn get_weather() -> FunctionTool {
    FunctionTool::new(
        "get_weather",
        "获取指定城市的天气信息"
    )
    .with_param("city", "城市名称", json!({"type": "string"}))
}
```

### 使用工具

```rust,ignore
use zai_rs::model::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = GLM4_5_flash {};
    let tool = get_weather();
    
    let messages = vec![
        TextMessage::system("你是一个有用的助手。"),
        TextMessage::user("北京今天的天气如何？"),
    ];
    
    let key = std::env::var("ZHIPU_API_KEY")?;
    
    let mut client = ChatCompletion::new(model, messages, key);
    
    // 添加工具
    client.body_mut().tools = Some(vec![tool]);
    
    let resp = client.post().await?;
    
    // 检查是否有工具调用
    if let Some(tool_calls) = resp.choices().first()
        .unwrap()
        .message
        .tool_calls() 
    {
        for call in tool_calls {
            println!("调用工具: {}", call.function().name());
            println!("参数: {}", call.function().arguments());
        }
    }
    
    Ok(())
}
```

### 使用 Toolkits

SDK 提供了 Toolkits 框架用于更高级的工具管理：

```rust,ignore
use zai_rs::toolkits::core::*;
use zai_rs::toolkits::executor::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let executor = ToolExecutor::new();
    
    // 注册工具
    executor.register_tool(get_weather())?;
    
    // 执行工具调用
    let tool_calls = vec![/* ... */];
    let results = executor.execute_all(tool_calls).await?;
    
    for (name, result) in results {
        println!("{}: {:?}", name, result);
    }
    
    Ok(())
}
```

## 异步聊天

### 概述

异步聊天 API 允许提交长时间运行的任务，稍后轮询获取结果。

### 提交异步任务

```rust,ignore
use zai_rs::model::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = GLM4_5_flash {};
    let messages = TextMessage::user("分析这段长文本...");
    let key = std::env::var("ZHIPU_API_KEY")?;
    
    let request = AsyncChatRequest::new(model, messages, key);
    let resp = request.post().await?;
    
    let task_id = resp.id().unwrap();
    println!("任务ID: {}", task_id);
    
    Ok(())
}
```

### 获取异步结果

```rust,ignore
use zai_rs::model::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = std::env::var("ZHIPU_API_KEY")?;
    let task_id = "your-task-id";
    
    let request = AsyncChatGetRequest::new(GLM4_5_flash {}, task_id, key);
    
    // 轮询直到完成
    loop {
        let resp = request.get().await?;
        
        match resp.task_status() {
            Some(TaskStatus::Success) => {
                println!("任务完成");
                if let Some(content) = resp.choices()
                    .and_then(|c| c.first())
                    .and_then(|c| c.message.content())
                {
                    println!("结果: {}", content);
                }
                break;
            }
            Some(TaskStatus::Processing) => {
                println!("处理中...");
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            Some(TaskStatus::Fail) => {
                eprintln!("任务失败");
                break;
            }
            _ => {
                eprintln!("未知状态");
                break;
            }
        }
    }
    
    Ok(())
}
```

## 文件管理

### 上传文件

```rust,ignore
use zai_rs::file::*;
use std::fs::File;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = std::env::var("ZHIPU_API_KEY")?;
    let path = "document.pdf";
    let purpose = FilePurpose::Batch;
    
    let file = File::new(path, purpose, key);
    let upload = file.upload().await?;
    
    println!("文件ID: {}", upload.id().unwrap());
    
    Ok(())
}
```

### 列出文件

```rust,ignore
use zai_rs::file::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = std::env::var("ZHIPU_API_KEY")?;
    
    let request = FileListRequest::new(key);
    let list = request.send().await?;
    
    for file in list.data() {
        println!("{}: {}", file.id().unwrap(), file.filename());
    }
    
    Ok(())
}
```

### 获取文件内容

```rust,ignore
use zai_rs::file::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = std::env::var("ZHIPU_API_KEY")?;
    let file_id = "file-abc123";
    
    let request = FileContentRequest::new(file_id, key);
    let content = request.send().await?;
    
    println!("文件内容: {}", content);
    
    Ok(())
}
```

### 删除文件

```rust,ignore
use zai_rs::file::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = std::env::var("ZHIPU_API_KEY")?;
    let file_id = "file-abc123";
    
    let request = FileDeleteRequest::new(file_id, key);
    request.send().await?;
    
    println!("文件已删除");
    
    Ok(())
}
```

## 知识库

### 创建知识库

```rust,ignore
use zai_rs::knowledge::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = std::env::var("ZHIPU_API_KEY")?;
    
    let request = CreateKnowledgeRequest::new(
        "我的知识库",
        "描述",
        key
    );
    
    let resp = request.send().await?;
    let kb_id = resp.id().unwrap();
    println!("知识库ID: {}", kb_id);
    
    Ok(())
}
```

### 上传文档

```rust,ignore
use zai_rs::knowledge::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = std::env::var("ZHIPU_API_KEY")?;
    let kb_id = "kb-abc123";
    let file_path = "document.pdf";
    
    let request = DocumentUploadFileRequest::new(
        kb_id,
        file_path,
        key
    );
    
    let resp = request.send().await?;
    println!("文档ID: {}", resp.document_id().unwrap());
    
    Ok(())
}
```

### 查询知识库

```rust,ignore
use zai_rs::knowledge::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = std::env::var("ZHIPU_API_KEY")?;
    let kb_id = "kb-abc123";
    let query = "什么是机器学习？";
    
    let request = RetrieveKnowledgeRequest::new(
        kb_id,
        query,
        key
    );
    
    let resp = request.send().await?;
    println!("相关内容: {:?}", resp);
    
    Ok(())
}
```

## 批量处理

### 创建批量任务

```rust,ignore
use zai_rs::batches::*;
use zai_rs::file::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = std::env::var("ZHIPU_API_KEY")?;
    
    // 1. 上传包含批量请求的文件
    let upload = FileUploadRequest::new(
        key.clone(),
        FilePurpose::Batch,
        "requests.jsonl"
    );
    let file: FileObject = upload.send().await?;
    let file_id = file.id().unwrap();
    
    // 2. 创建批量任务
    let create = CreateBatchRequest::new(
        key.clone(),
        file_id,
        BatchEndpoint::ChatCompletions
    )
    .with_completion_window("24h");
    
    let resp: CreateBatchResponse = create.send().await?;
    let batch_id = resp.id().unwrap();
    println!("批量任务ID: {}", batch_id);
    
    Ok(())
}
```

### 检查批量任务状态

```rust,ignore
use zai_rs::batches::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = std::env::var("ZHIPU_API_KEY")?;
    let batch_id = "batch-abc123";
    
    let request = RetrieveBatchRequest::new(batch_id, key);
    let batch: Batch = request.send().await?;
    
    println!("状态: {:?}", batch.status());
    println!("完成数: {}", batch.request_counts().completed());
    
    Ok(())
}
```

### 取消批量任务

```rust,ignore
use zai_rs::batches::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = std::env::var("ZHIPU_API_KEY")?;
    let batch_id = "batch-abc123";
    
    let request = CancelBatchRequest::new(batch_id, key);
    request.send().await?;
    
    println!("批量任务已取消");
    
    Ok(())
}
```

## 性能优化

### 1. 连接复用

SDK 使用单例 HTTP 客户端，自动复用连接：

```rust,ignore
// ✅ 自动连接复用
for i in 0..10 {
    let client = ChatCompletion::new(model, messages, key);
    let resp = client.post().await?;
}
```

### 2. 并发请求

使用 Tokio 进行并发处理：

```rust,ignore
use futures::future::join_all;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let queries = vec!["问题1", "问题2", "问题3"];
    let key = std::env::var("ZHIPU_API_KEY")?;
    
    let futures: Vec<_> = queries.into_iter()
        .map(|q| {
            let client = ChatCompletion::new(
                GLM4_5_flash {},
                TextMessage::user(q),
                key.clone()
            );
            client.post()
        })
        .collect();
    
    let results = join_all(futures).await;
    
    for result in results {
        println!("{:?}", result);
    }
    
    Ok(())
}
```

### 3. 合理设置超时

```rust,ignore
// 注意：当前版本不支持自定义超时，使用默认 60 秒
// 未来版本将支持 HttpClientConfig 自定义
```

## 安全最佳实践

### 1. 保护 API 密钥

```rust,ignore
// ✅ 从环境变量读取
let key = std::env::var("ZHIPU_API_KEY")?;

// ❌ 不要硬编码
let key = "sk-abc123.xyz";
```

### 2. 使用敏感信息过滤

```rust,ignore
use zai_rs::client::error::mask_sensitive_info;

fn log_request(api_key: &str, content: &str) {
    let log = format!("Key: {}, Content: {}", api_key, content);
    println!("{}", mask_sensitive_info(&log));
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
