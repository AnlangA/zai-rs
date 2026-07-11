use zai_rs::client::ZaiClient;
use zai_rs::model::chat_base_response::TaskStatus;
use zai_rs::model::{async_chat::*, async_chat_get::*, *};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let message = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "hello".to_string());
    let request =
        AsyncChatCompletion::new(GLM4_5 {}, TextMessage::user(&message)).with_temperature(0.7);
    let body = request.send_via(&client).await?;
    if let Some(task_id) = body.id() {
        println!("Task: {task_id}");
        let get = AsyncChatGetRequest::new(GLM4_5 {}, task_id.to_string());
        loop {
            let result = get.send_via(&client).await?;
            match result.task_status() {
                Some(TaskStatus::Success) => {
                    println!("Done: {result:#?}");
                    break;
                },
                Some(TaskStatus::Fail) => {
                    eprintln!("Failed");
                    break;
                },
                _ => {
                    println!("Processing...");
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                },
            }
        }
    }
    Ok(())
}
