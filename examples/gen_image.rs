//! Generate one square image with CogView.

use zai_rs::{
    client::ZaiClient,
    model::gen_image::{CogView4, ImageGenRequest, ImageResponse, ImageSize},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let prompt = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "一只坐在阳光窗台上的猫".to_owned());

    let client = ZaiClient::from_env()?;
    let request = ImageGenRequest::new(CogView4 {})
        .with_prompt(prompt)
        .with_size(ImageSize::Size1024x1024);

    let body: ImageResponse = request.send_via(&client).await?;
    println!("{body:#?}");

    Ok(())
}
