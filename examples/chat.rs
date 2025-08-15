use tokio;
use zai_rs::model::base::*;
use zai_rs::model::chat::data::ChatCompletion;
use zai_rs::client::http::*;
#[tokio::main]
async fn main() {
    let model = GLM4_5_flash {};

    let (url, mut body) = ChatCompletion::new(model, TextMessage::user("你好"));
    body = body.with_thinking(ThinkingType::Disabled);
    let json = serde_json::to_string_pretty(&body).unwrap();
    let key = get_key();
    let client = HttpClient::new(key, url, json);
    let response = client.post().await.unwrap();
    let text = response.text().await.unwrap();
    println!("Response: {}", text);
}

fn get_key() -> String {
    if let Ok(key) = std::env::var("ZHIPU_API_KEY") {
        key
    } else {
        // 从输入中读取
        let mut key = String::new();
        std::io::stdin().read_line(&mut key).unwrap();
        key.trim().to_string()
    }
}
