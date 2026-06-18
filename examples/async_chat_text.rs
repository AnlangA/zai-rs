use zai_rs::{
    client::http::*,
    model::{
        async_chat::AsyncChatCompletion,
        async_chat_get::AsyncChatGetRequest,
        chat_base_response::{ChatCompletionResponse, TaskStatus},
        *,
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

    // 等待并获取结果
    tracing::trace!("\n=== 获取异步聊天结果 ===");
    for (message, task_id) in task_ids {
        tracing::trace!("问题: {}", message);

        // 轮询直到完成
        let request = AsyncChatGetRequest::new(GLM4_5 {}, task_id, key.clone());
        loop {
            let result = async {
                let resp = request
                    .get()
                    .await
                    .map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?;
                resp.json::<ChatCompletionResponse>()
                    .await
                    .map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))
            }
            .await;

            match result {
                Ok(body) => match body.task_status() {
                    Some(TaskStatus::Success) => {
                        tracing::trace!("状态: 完成");
                        if let Some(content) = body
                            .choices()
                            .and_then(|choices| choices.first())
                            .and_then(|choice| choice.message.content())
                        {
                            tracing::trace!("回复: {}", content)
                        }
                        break;
                    },
                    Some(TaskStatus::Fail) => {
                        tracing::trace!("状态: 失败");
                        break;
                    },
                    Some(TaskStatus::Processing) => {
                        tracing::trace!("状态: 处理中...");
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    },
                    None => {
                        tracing::trace!("状态: 未知");
                        break;
                    },
                },
                Err(e) => {
                    tracing::trace!("获取结果失败: {}", e);
                    break;
                },
            }
        }
        tracing::trace!("---");
    }

    Ok(())
}
