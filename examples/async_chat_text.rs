//! Submit an asynchronous chat request and poll its bounded task lifecycle.

use std::time::Duration;

use zai_rs::{
    client::ZaiClient,
    model::{
        AsyncTaskGetRequest, AsyncTaskResult, GLM5_2, TextMessage, async_chat::AsyncChatCompletion,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let message = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "用一句话介绍 Rust。".to_owned());

    let client = ZaiClient::from_env()?;
    let request = AsyncChatCompletion::new(GLM5_2 {}, TextMessage::user(message));
    let body = request.send_via(&client).await?;
    let task_id = body.id().ok_or("response did not contain a task id")?;
    println!("submitted task {task_id}");

    let get = AsyncTaskGetRequest::new(task_id);
    tokio::time::timeout(Duration::from_secs(300), async {
        loop {
            let result = get.send_via(&client).await?;
            match result {
                AsyncTaskResult::Chat(result) => {
                    println!("{result:#?}");
                    return Ok::<_, Box<dyn std::error::Error>>(());
                },
                AsyncTaskResult::State(state) if state.is_failed() => {
                    return Err("asynchronous chat task failed".into());
                },
                AsyncTaskResult::State(state) if state.is_success() => {
                    return Err("chat task succeeded without a chat result".into());
                },
                AsyncTaskResult::State(_) => tokio::time::sleep(Duration::from_secs(2)).await,
                AsyncTaskResult::Video(_) | AsyncTaskResult::Image(_) => {
                    return Err("chat task returned an unexpected media result".into());
                },
            }
        }
    })
    .await??;

    Ok(())
}
