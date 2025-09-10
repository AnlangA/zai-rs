use zai_rs::model::async_chat_get::AsyncChatGetRequest;
use zai_rs::model::chat_base_response::ChatCompletionResponse;
use zai_rs::model::chat_base_response::TaskStatus;
use zai_rs::model::gen_video_async::*;

use tokio;
use zai_rs::client::http::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let model = CogVideoX3 {};
    let key = std::env::var("ZHIPU_API_KEY").unwrap();
    println!("{key}");
    let user_text = "可爱小猫叠在一起";
    
    // 提交视频生成请求
    let client = VideoGenRequest::new(model, key.clone()).with_prompt(user_text);
    let resp = client.post().await?;
    let body: ChatCompletionResponse = resp.json().await?;
    
    let task_id = body.id().unwrap();
    println!("Task ID: {}", task_id);
    
    // 使用 async_chat_get 轮询结果
    let get_request = AsyncChatGetRequest::new(CogVideoX3 {}, task_id.to_string(), key);
    
    loop {
        let get_resp = get_request.get().await?;
        let get_body: ChatCompletionResponse = get_resp.json().await?;
        
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
            }
            Some(TaskStatus::Fail) => {
                println!("Video generation failed!");
                break;
            }
            Some(TaskStatus::Processing) => {
                println!("Processing...");
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            }
            None => {
                println!("No task status found");
                break;
            }
        }
    }
    
    Ok(())
}
