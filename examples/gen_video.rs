use zai_rs::{
    client::ZaiClient,
    model::{
        async_chat_get::AsyncChatGetRequest, chat_base_response::TaskStatus, gen_video_async::*,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = CogVideoX3 {};
    let client = ZaiClient::from_env()?;
    let user_text = "可爱小猫叠在一起";

    // 提交视频生成请求 (P05: credentials live on the ZaiClient).
    let request = VideoGenRequest::new(model).with_prompt(user_text);
    let body = request.send_via(&client).await?;

    let task_id = body.id().ok_or("Task ID not found in response")?;
    println!("Task ID: {task_id}");

    // 使用 async_chat_get 轮询结果
    let get_request = AsyncChatGetRequest::new(CogVideoX3 {}, task_id.to_string());

    loop {
        let get_body = get_request.send_via(&client).await?;

        match get_body.task_status() {
            Some(TaskStatus::Success) => {
                println!("Video generation completed!");
                if let Some(video_result) = get_body.video_result() {
                    for video in video_result {
                        println!("Video URL: {:?}", video.url());
                        println!("Cover Image: {:?}", video.cover_image_url());
                    }
                }
                break;
            },
            Some(TaskStatus::Fail) => {
                eprintln!("Video generation failed!");
                break;
            },
            Some(TaskStatus::Processing) => {
                println!("Processing...");
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            },
            // `TaskStatus` is `#[non_exhaustive]`; `Some(_)` covers `Unknown`
            // (an unrecognized value from a newer API) and any future variant —
            // keep polling.
            Some(_) => {
                println!("Unrecognized task status; continuing to poll...");
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            },
            None => {
                eprintln!("No task status found");
                break;
            },
        }
    }

    Ok(())
}
