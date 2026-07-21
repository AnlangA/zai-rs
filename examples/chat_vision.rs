//! Ask a vision model about an image.
//!
//! Defaults to the local `data/短发女.jpeg` (sent as a base64 data URL);
//! pass an HTTPS image URL to use a remote image instead.

use base64::Engine;
use zai_rs::{
    client::ZaiClient,
    model::{
        GLM4_6v, VisionMessage, VisionRichContent, chat::ChatCompletion,
        chat_base_response::ChatCompletionResponse,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let image_url = match std::env::args().nth(1) {
        Some(url) => url,
        None => {
            let bytes = tokio::fs::read("data/短发女.jpeg").await?;
            let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
            format!("data:image/jpeg;base64,{encoded}")
        },
    };

    let client = ZaiClient::from_env()?;
    let vision_message = VisionMessage::new_user()
        .add_content(VisionRichContent::image(image_url))
        .add_content(VisionRichContent::text("请用中文描述这张图像。"));
    let request = ChatCompletion::new(GLM4_6v {}, vision_message);

    let body: ChatCompletionResponse = request.send_via(&client).await?;
    println!("{body:#?}");
    Ok(())
}
