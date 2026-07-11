//! # Translation Bot Example (non-streaming)
//!
//! P05 note: the SSE streaming path is rebuilt in P08; this example uses the
//! non-streaming path for now.

use std::io::{self, Write};

use zai_rs::client::ZaiClient;
use zai_rs::model::{chat_base_response::ChatCompletionResponse, *};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let model = GLM4_5_flash {};

    println!("翻译机器人 (输入 exit 退出)");
    loop {
        print!("原文> ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let prompt = input.trim();
        if prompt.eq_ignore_ascii_case("exit") || prompt.is_empty() {
            break;
        }

        let system = TextMessage::system(
            "你是一个专业的翻译助手。请将用户提供的文本翻译成英文。只返回翻译结果。",
        );
        let request = ChatCompletion::new(model.clone(), system)
            .add_messages(TextMessage::user(prompt))
            .with_temperature(0.3)
            .with_thinking(ThinkingType::disabled());

        let body: ChatCompletionResponse = request.send_via(&client).await?;
        let text = body
            .choices()
            .and_then(|cs| cs.first())
            .and_then(|c| c.message().content())
            .map(|v| v.as_str().unwrap_or("").to_string())
            .unwrap_or_default();
        println!("译文> {text}\n");
    }
    Ok(())
}
