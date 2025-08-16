use tokio;
use zai_rs::model::base::*;
use zai_rs::model::chat::data::ChatCompletion;
use zai_rs::client::http::*;
#[tokio::main]
async fn main() {
    let model = GLM4_5_flash {};

    // 模拟添加一个 function call 工具：get_weather(city: string)
    let weather_func = Function::new(
        "get_weather",
        "Get current weather for a city",
        serde_json::json!({
            "type": "object",
            "properties": {
                "city": {"type": "string"}
            },
            "required": ["city"],
            "additionalProperties": false
        }),
    );
    let tools = Tools::Function { function: weather_func };

    let client = ChatCompletion::new(model, TextMessage::user("你是谁，帮为查找深圳今天的天气"), get_key())
        .with_thinking(ThinkingType::Disabled)
        .with_temperature(0.7)
        .with_top_p(0.9)
        .with_max_tokens(512)
        .with_tools(vec![tools]);

    let resp = client.post().await.unwrap();
    let v: serde_json::Value = resp.json().await.unwrap();
    println!("{}", serde_json::to_string_pretty(&v).unwrap());
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
