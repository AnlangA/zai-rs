use zai_rs::model::chat_base_response::ChatCompletionResponse;
use zai_rs::model::*;

use tokio;
use zai_rs::client::http::*;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let model = GLM4_5_flash {};
    let key = std::env::var("ZHIPU_API_KEY").unwrap();
    let user_text = "你好";
    let client = ChatCompletion::new(model, TextMessage::user(user_text), key)
        .with_temperature(0.7)
        .with_top_p(0.9)
        .with_thinking(ThinkingType::Disabled);
    let resp = client.post().await.unwrap();
    let body: ChatCompletionResponse = resp.json().await.unwrap();
    println!("{:#?}", body);
    Ok(())
}
