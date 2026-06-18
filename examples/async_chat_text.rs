use zai_rs::{
    client::http::*,
    model::{
        async_chat::AsyncChatCompletion, async_chat_get::AsyncChatGetRequest,
        chat_base_response::TaskStatus, *,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let enable_logging = std::env::var_os("RUST_LOG").is_some();
    if enable_logging {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .try_init();
    }

    // 获取API密钥
    let key =
        std::env::var("ZHIPU_API_KEY").expect("ZHIPU_API_KEY environment variable must be set");

    // 提交异步聊天任务
    tracing::trace!("=== 提交异步聊天任务 ===");
    let messages = vec![
        "你好，请介绍一下机器学习的基本概念",
        "你能解释一下什么是深度学习吗？",
        "请简单说明自然语言处理的应用场景",
    ];

    let mut task_ids = vec![];

    for message in messages {
        let key_clone = key.clone();
        let http_config = HttpClientConfigBuilder::default()
            .logging(enable_logging)
            .mask_sensitive_data(false)
            .build();
        let client = AsyncChatCompletion::new(GLM4_5 {}, TextMessage::user(message), key_clone)
            .with_temperature(0.7)
            .with_top_p(0.9)
            .with_http_config(http_config);

        match client.send().await {
            Ok(body) => {
                if let Some(task_id) = body.id() {
                    tracing::trace!("问题: {}", message);
                    tracing::trace!("任务ID: {}", task_id);
                    task_ids.push((message, task_id.to_string()));
                }
            },
            Err(e) => {
                tracing::trace!("提交失败: {}", e);
            },
        }
    }

    // 等待并获取结果。
    //
    // 并发轮询所有任务：每个任务在一个独立的 spawned task 里各自轮询到终态，
    // 多个任务并行推进（服务端也是并行处理的）。总耗时 ≈ max(单任务)，
    // 而非串行场景下的 Σ(单任务)。
    tracing::trace!("\n=== 获取异步聊天结果 ===");
    use tokio::task::JoinSet;

    let mut set: JoinSet<(&'static str, Result<String, String>)> = JoinSet::new();
    for (message, task_id) in task_ids {
        let key = key.clone();
        set.spawn(async move {
            let request = AsyncChatGetRequest::new(GLM4_5 {}, task_id, key);
            loop {
                // 用 request.send()（内部经 parse_typed_response），与 POST 路径走同一条
                // `trace!` 管道，GET 响应体同样会被记录。
                match request.send().await {
                    Ok(body) => match body.task_status() {
                        Some(TaskStatus::Success) => {
                            // content 是 JSON Value，常见为 {"content": "..."} 或字符串。
                            let content = body
                                .choices()
                                .and_then(|choices| choices.first())
                                .and_then(|choice| choice.message.content())
                                .map(|v| match v {
                                    serde_json::Value::String(s) => s.clone(),
                                    other => other.to_string(),
                                })
                                .unwrap_or_default();
                            break (message, Ok(content));
                        },
                        Some(TaskStatus::Fail) => {
                            break (message, Err("任务失败".to_string()));
                        },
                        Some(TaskStatus::Processing) => {
                            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                        },
                        None => break (message, Err("未知状态".to_string())),
                    },
                    Err(e) => break (message, Err(format!("获取结果失败: {e}"))),
                }
            }
        });
    }

    while let Some(res) = set.join_next().await {
        match res {
            Ok((message, Ok(content))) => {
                tracing::trace!("问题: {message}");
                tracing::trace!("回复: {content}");
                tracing::trace!("---");
            },
            Ok((message, Err(e))) => {
                tracing::trace!("问题: {message}");
                tracing::trace!("{e}");
                tracing::trace!("---");
            },
            Err(e) => tracing::trace!("轮询任务异常: {e}"),
        }
    }

    Ok(())
}
