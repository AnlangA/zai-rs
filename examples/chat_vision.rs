use zai_rs::client::v2::ZaiClient;
use zai_rs::model::{chat_base_response::ChatCompletionResponse, *};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The media URL is read from the first CLI argument instead of a hardcoded
    // (and, in the prior revision, expired/signed) URL. Missing the argument
    // prints usage and exits 2 — never a panic.
    let media_url = match std::env::args().nth(1) {
        Some(url) => url,
        None => {
            eprintln!("usage: chat_vision <media-url>");
            eprintln!("       pass an https URL to an image or video for the vision model");
            std::process::exit(2);
        },
    };

    let model = GLM4_5v {};
    let client = ZaiClient::from_env()?;

    // Create video content from the user-provided media URL.
    let video_content = VisionRichContent::video(&media_url);
    let text_content = VisionRichContent::text("这个视频描述了什么?，用中文回复我");
    let vision_message = VisionMessage::new_user()
        .add_content(video_content)
        .add_content(text_content);
    let request = ChatCompletion::new(model, vision_message);

    let body: ChatCompletionResponse = request.send_via(&client).await?;
    println!("{body:#?}");
    Ok(())
}
