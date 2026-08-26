//! Ask GLM-5.3-Flash about the bundled MP4 video.
//!
//! The example reads `data/长发女听歌.mp4`, encodes it as Base64, and sends the
//! encoded video directly in the `video_url.url` field.

use base64::Engine;
use zai_rs::{
    client::ZaiClient,
    model::{
        GLM5_3_flash, ReasoningEffort, ThinkingType, VisionMessage, VisionRichContent,
        chat::ChatCompletion, chat_base_response::ChatCompletionResponse,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let video = tokio::fs::read("data/长发女听歌.mp4").await?;
    let video_base64 = base64::engine::general_purpose::STANDARD.encode(video);

    let client = ZaiClient::from_env()?;
    let message = VisionMessage::new_user()
        .add_content(VisionRichContent::video(video_base64))
        .add_content(VisionRichContent::text(
            "请用中文概括这段视频的主要内容，并按时间顺序列出关键事件。",
        ));
    let request = ChatCompletion::new(GLM5_3_flash {}, message)
        .with_thinking(ThinkingType::enabled().with_clear_thinking(false))
        .with_reasoning_effort(ReasoningEffort::Max);

    let body: ChatCompletionResponse = request.send_via(&client).await?;
    println!("{body:#?}");
    Ok(())
}
