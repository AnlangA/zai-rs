use zai_rs::model::*;
use zai_rs::model::base_requst::*;
use zai_rs::model::base_response::ChatCompletionResponse;

use zai_rs::client::http::*;
use tokio;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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