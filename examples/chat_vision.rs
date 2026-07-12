//! Ask a vision model about an image available at an HTTPS URL.

use zai_rs::{
    client::ZaiClient,
    model::{
        GLM4_6v, VisionMessage, VisionRichContent, chat::ChatCompletion,
        chat_base_response::ChatCompletionResponse,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let image_url = std::env::args()
        .nth(1)
        .ok_or("usage: chat_vision <https-image-url>")?;

    let client = ZaiClient::from_env()?;
    let vision_message = VisionMessage::new_user()
        .add_content(VisionRichContent::image(image_url))
        .add_content(VisionRichContent::text("请用中文描述这张图像。"));
    let request = ChatCompletion::new(GLM4_6v {}, vision_message);

    let body: ChatCompletionResponse = request.send_via(&client).await?;
    println!("{body:#?}");
    Ok(())
}
