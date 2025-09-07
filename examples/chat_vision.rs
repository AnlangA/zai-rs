use zai_rs::model::chat_base_response::ChatCompletionResponse;
use zai_rs::model::*;

use tokio;
use zai_rs::client::http::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = GLM4_5v {};
    let key = std::env::var("ZHIPU_API_KEY").unwrap();

    // Create video content from the local file
    let video_content = VisionRichContent::video("https://maas-watermark-prod.cn-wlcb.ufileos.com/1757254909722_watermark.mp4?UCloudPublicKey=TOKEN_75a9ae85-4f15-4045-940f-e94c0f82ae90&Signature=zYLD3mC%2FxDlL%2F4N%2FuDQJQ%2Fp%2F5%2BI%3D&Expires=1757341309");
    let vision_message = VisionMessage::user(video_content);

    let client = ChatCompletion::new(model, vision_message, key)
        .with_temperature(0.7)
        .with_top_p(0.9);

    let resp = client.post().await.unwrap();
    let body: ChatCompletionResponse = resp.json().await.unwrap();
    println!("{:#?}", body);
    Ok(())
}
