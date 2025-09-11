use zai_rs::client::http::*;
use zai_rs::model::gen_image::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Prepare API key and model
    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");
    let model = CogView4 {};

    // Example prompt and size (equivalent to the curl example)
    let prompt = "一只可爱的小猫咪，坐在阳光明媚的窗台上，背景是蓝天白云.";
    let size = ImageSize::Size1024x1024;

    // Build request and send
    let client = ImageGenRequest::new(model, key)
        .with_prompt(prompt)
        .with_size(size);

    let resp = client.post().await?;
    let body: ImageResponse = resp.json().await?;

    println!("{:#?}", body);

    Ok(())
}
