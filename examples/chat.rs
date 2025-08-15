
use zai_rs::model::base::*;
use zai_rs::model::chat::data::ChatCompletion;
use tokio;
use url::Url;
#[tokio::main]
async fn main() {
    let model = GLM4_5{};

    let (url, body) = ChatCompletion::new(model, TextMessage::user("Hello, how are you?"));

    let json = serde_json::to_string_pretty(&body).unwrap();
    println!("ChatCompletion 序列化结果:");
    println!("{}\n", json);
}