use zai_rs::model::{chat_base_response::ChatCompletionResponse, *};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let model = GLM4_5v {};
    let key =
        std::env::var("ZHIPU_API_KEY").expect("ZHIPU_API_KEY environment variable must be set");

    // Create video content from the local file
    let video_content = VisionRichContent::video(
        "https://maas-watermark-prod.cn-wlcb.ufileos.com/1757254909722_watermark.mp4?UCloudPublicKey=TOKEN_75a9ae85-4f15-4045-940f-e94c0f82ae90&Signature=zYLD3mC%2FxDlL%2F4N%2FuDQJQ%2Fp%2F5%2BI%3D&Expires=1757341309",
    );
    let text_content = VisionRichContent::text("这个视频描述了什么?，用中文回复我");
    let vision_message = VisionMessage::new_user()
        .add_user(video_content)
        .add_user(text_content);
    let client = ChatCompletion::new(model, vision_message, key);

    let body: ChatCompletionResponse = client.send().await?;
    println!("{:#?}", body);
    Ok(())
}
