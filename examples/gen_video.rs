//! Submit a video-generation task and poll it with an overall timeout.

use std::time::Duration;

use zai_rs::{
    client::ZaiClient,
    model::{
        AsyncTaskGetRequest, AsyncTaskResult, TaskStatus,
        gen_video_async::{CogVideoX3, VideoGenRequest},
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let prompt = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "几只小猫依偎在窗边".to_owned());

    let client = ZaiClient::from_env()?;
    let submitted = VideoGenRequest::new(CogVideoX3 {})
        .with_prompt(prompt)
        .send_via(&client)
        .await?;
    let task_id = submitted.id().ok_or("response did not contain a task id")?;
    println!("submitted task {task_id}");

    let get = AsyncTaskGetRequest::new(task_id);
    let completed = tokio::time::timeout(Duration::from_secs(600), async {
        loop {
            let response = get.send_via(&client).await?;
            match response {
                AsyncTaskResult::Video(result)
                    if result.videos().is_some_and(|videos| !videos.is_empty()) =>
                {
                    return Ok::<_, Box<dyn std::error::Error>>(result);
                },
                AsyncTaskResult::Video(result)
                    if matches!(result.status(), Some(TaskStatus::Fail)) =>
                {
                    return Err("video-generation task failed".into());
                },
                AsyncTaskResult::Video(result)
                    if matches!(result.status(), Some(TaskStatus::Success)) =>
                {
                    return Err("video task succeeded without a video result".into());
                },
                AsyncTaskResult::Video(_) => {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                },
                AsyncTaskResult::State(state) if state.is_failed() => {
                    return Err("video-generation task failed".into());
                },
                AsyncTaskResult::State(state) if state.is_success() => {
                    return Err("video task succeeded without a video result".into());
                },
                AsyncTaskResult::State(_) => tokio::time::sleep(Duration::from_secs(5)).await,
                AsyncTaskResult::Chat(_) | AsyncTaskResult::Image(_) => {
                    return Err("video task returned an unexpected result type".into());
                },
            }
        }
    })
    .await??;

    let videos = completed
        .videos()
        .filter(|videos| !videos.is_empty())
        .ok_or("completed task did not contain a video result")?;
    for video in videos {
        if let Some(url) = video.url() {
            println!("video: {url}");
        }
        if let Some(url) = video.cover_image_url() {
            println!("cover: {url}");
        }
    }

    Ok(())
}
