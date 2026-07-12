//! Moderate a single text input.

use zai_rs::{client::ZaiClient, model::moderation::Moderation};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let text = "这是一段需要审核的文本";
    let result = Moderation::new_text(text).send_via(&client).await?;
    println!("{result:#?}");
    Ok(())
}
