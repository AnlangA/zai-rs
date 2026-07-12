//! Maintain conversation history across an interactive non-streaming chat.

use std::io::{self, Write};

use zai_rs::{
    client::ZaiClient,
    model::{
        GLM4_5_airx, TextMessage, ThinkingType, chat::ChatCompletion,
        chat_base_response::ChatCompletionResponse,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let mut request = ChatCompletion::new(
        GLM4_5_airx {},
        TextMessage::system("你是一个简洁、可靠的中文助手。"),
    )
    .with_temperature(0.7)
    .with_top_p(0.9)
    .with_thinking(ThinkingType::disabled());
    let mut line = String::new();

    println!("输入 exit 或 quit 退出。\n");
    loop {
        print!("你> ");
        io::stdout().flush()?;
        line.clear();
        if io::stdin().read_line(&mut line)? == 0 {
            break;
        }

        let user_input = line.trim();
        if user_input.eq_ignore_ascii_case("exit") || user_input.eq_ignore_ascii_case("quit") {
            break;
        }
        if user_input.is_empty() {
            continue;
        }

        request = request.add_message(TextMessage::user(user_input));
        let body: ChatCompletionResponse = request.send_via(&client).await?;
        let answer = body
            .choices()
            .and_then(|choices| choices.first())
            .and_then(|choice| choice.message())
            .and_then(|message| message.content_str())
            .ok_or("chat response did not contain text")?
            .to_owned();

        println!("AI> {answer}\n");
        request = request.add_message(TextMessage::assistant(answer));
    }

    Ok(())
}
