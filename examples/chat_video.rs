//! Ask GLM-5.3-Flash about the bundled MP4 video.
//!
//! The example reads `data/长发女听歌.mp4`, encodes it as Base64, and sends the
//! encoded video directly in the `video_url.url` field.
//! Video upload and understanding can exceed the SDK's default 60-second
//! attempt deadline, so this example uses a ten-minute single-attempt policy
//! and streams the response as it is generated.

use std::io::Write;
use std::time::Duration;

use base64::Engine;
use zai_rs::{
    client::{RequestOptions, ZaiClient},
    model::{
        GLM5_3_flash, ReasoningEffort, ThinkingType, VisionMessage, VisionRichContent,
        chat::ChatCompletion,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let video = tokio::fs::read("data/长发女听歌.mp4").await?;
    let video_base64 = base64::engine::general_purpose::STANDARD.encode(video);

    let video_timeout = Duration::from_secs(10 * 60);
    let client = ZaiClient::from_env()?.with_request_options(
        RequestOptions::default()
            .with_attempt_timeout(video_timeout)?
            .with_overall_timeout(video_timeout)?
            .with_sse_handshake_timeout(video_timeout)?
            .with_sse_idle_timeout(video_timeout)?
            .with_max_attempts(1)?,
    );
    let message = VisionMessage::new_user()
        .add_content(VisionRichContent::video(video_base64))
        .add_content(VisionRichContent::text(
            "请用中文概括这段视频的主要内容，并按时间顺序列出关键事件。",
        ));
    let request = ChatCompletion::new(GLM5_3_flash {}, message)
        .with_thinking(ThinkingType::enabled().with_clear_thinking(false))
        .with_reasoning_effort(ReasoningEffort::Max);

    let mut stream = request.enable_stream().stream_via(&client).await?;
    while let Some(chunk) = stream.next().await {
        for choice in chunk?.choices {
            if let Some(content) = choice.delta.and_then(|delta| delta.content) {
                print!("{content}");
                std::io::stdout().flush()?;
            }
        }
    }
    println!();
    Ok(())
}
