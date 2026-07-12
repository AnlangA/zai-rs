//! Chat via the Coding Plan endpoint using `ZaiClient`.
use zai_rs::client::ZaiClient;
use zai_rs::model::{chat_base_response::ChatCompletionResponse, *};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = GLM4_5 {};
    let client = ZaiClient::from_env()?;
    let user_text = "写一个 Rust hello world";

    let request = ChatCompletion::new(model, TextMessage::user(user_text));
    let body: ChatCompletionResponse = request.send_via_coding_plan(&client).await?;
    println!("{body:#?}");
    Ok(())
}
