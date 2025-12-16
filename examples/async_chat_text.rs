use tokio;
use zai_rs::client::http::*;
use zai_rs::model::async_chat::AsyncChatCompletion;
use zai_rs::model::async_chat_get::AsyncChatGetRequest;
use zai_rs::model::chat_base_response::ChatCompletionResponse;
use zai_rs::model::chat_base_response::TaskStatus;
use zai_rs::model::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // 获取API密钥
    let key = std::env::var("ZHIPU_API_KEY").unwrap();

    // 提交异步聊天任务
    println!("=== 提交异步聊天任务 ===");
    let messages = vec![
        "你好，请介绍一下机器学习的基本概念",
        "你能解释一下什么是深度学习吗？",
        "请简单说明自然语言处理的应用场景",
    ];

    let mut task_ids = vec![];

    for message in messages {
        let key_clone = key.clone();
        let client = AsyncChatCompletion::new(GLM4_5 {}, TextMessage::user(message), key_clone)
            .with_temperature(0.7)
            .with_top_p(0.9);

        match client.send().await {
            Ok(body) => {
                if let Some(task_id) = body.id() {
                    println!("问题: {}", message);
                    println!("任务ID: {}", task_id);
                    task_ids.push((message, task_id.to_string()));
                }
            }
            Err(e) => {
                println!("提交失败: {}", e);
            }
        }
    }

    // 等待并获取结果
    println!("\n=== 获取异步聊天结果 ===");
    for (message, task_id) in task_ids {
        println!("问题: {}", message);

        // 轮询直到完成
        let request = AsyncChatGetRequest::new(GLM4_5 {}, task_id, key.clone());
        loop {
            let result: Result<ChatCompletionResponse, Box<dyn std::error::Error>> = async {
                let resp = request.get().await?;
                let json = resp.json::<ChatCompletionResponse>().await?;
                Ok(json)
            }
            .await;

            match result {
                Ok(body) => match body.task_status() {
                    Some(TaskStatus::Success) => {
                        println!("状态: 完成");
                        if let Some(choices) = body.choices() {
                            if let Some(choice) = choices.first() {
                                if let Some(content) = choice.message.content() {
                                    println!("回复: {}", content);
                                }
                            }
                        }
                        break;
                    }
                    Some(TaskStatus::Fail) => {
                        println!("状态: 失败");
                        break;
                    }
                    Some(TaskStatus::Processing) => {
                        println!("状态: 处理中...");
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    }
                    None => {
                        println!("状态: 未知");
                        break;
                    }
                },
                Err(e) => {
                    println!("获取结果失败: {}", e);
                    break;
                }
            }
        }
        println!("---");
    }

    Ok(())
}
