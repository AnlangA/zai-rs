//! Translate interactive input while keeping each request independent.

use std::io::{self, Write};

use zai_rs::{
    client::ZaiClient,
    model::{
        GLM4_5_flash, TextMessage, ThinkingType, chat::ChatCompletion,
        chat_base_response::ChatCompletionResponse,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let model = GLM4_5_flash {};

    println!("翻译机器人 (输入 exit 退出)");
    loop {
        print!("原文> ");
        io::stdout().flush()?;
        let mut input = String::new();
        if io::stdin().read_line(&mut input)? == 0 {
            break;
        }
        let prompt = input.trim();
        if prompt.eq_ignore_ascii_case("exit") || prompt.is_empty() {
            break;
        }

        let system = TextMessage::system(
            "你是一个专业的翻译助手。请将用户提供的文本翻译成英文。只返回翻译结果。",
        );
        let request = ChatCompletion::new(model, system)
            .add_message(TextMessage::user(prompt))
            .with_temperature(0.3)
            .with_thinking(ThinkingType::disabled());

        let body: ChatCompletionResponse = request.send_via(&client).await?;
        let text = body
            .choices()
            .and_then(|cs| cs.first())
            .and_then(|choice| choice.message())
            .and_then(|message| message.content_str())
            .ok_or("translation response did not contain text")?;
        println!("译文> {text}\n");
    }
    Ok(())
}
