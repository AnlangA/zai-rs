use std::io::{self, Write};

use zai_rs::client::v2::ZaiClient;
use zai_rs::model::{chat_base_response::ChatCompletionResponse, *};

fn extract_text_from_content(v: &serde_json::Value) -> Option<String> {
    v.as_str().map(std::string::ToString::to_string)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = GLM4_5_airx {};
    let client = ZaiClient::from_env()?;

    println!("可持续对话示例 (输入 exit 或 quit 退出)\n");

    let mut line = String::new();

    print!("你> ");
    io::stdout().flush().ok();
    line.clear();
    io::stdin().read_line(&mut line)?;
    let mut user_input = line.trim().to_string();
    if user_input.eq_ignore_ascii_case("exit") || user_input.eq_ignore_ascii_case("quit") {
        return Ok(());
    }

    let mut request = ChatCompletion::new(model, TextMessage::user(&user_input))
        .with_temperature(0.7)
        .with_top_p(0.9)
        .with_thinking(ThinkingType::disabled());

    loop {
        let body: ChatCompletionResponse = request.send_via(&client).await?;

        let ai_text = body
            .choices()
            .and_then(|cs| cs.first())
            .and_then(|c| c.message().content())
            .and_then(extract_text_from_content)
            .unwrap_or_else(|| "<empty>".to_string());

        println!("AI> {ai_text}\n");

        request = request.add_messages(TextMessage::assistant(ai_text));

        print!("你> ");
        io::stdout().flush().ok();
        line.clear();
        io::stdin().read_line(&mut line)?;
        user_input = line.trim().to_string();
        if user_input.eq_ignore_ascii_case("exit") || user_input.eq_ignore_ascii_case("quit") {
            break;
        }
        if user_input.is_empty() {
            continue;
        }

        request = request.add_messages(TextMessage::user(&user_input));
    }

    Ok(())
}
