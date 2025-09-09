use zai_rs::model::chat_base_response::ChatCompletionResponse;
use zai_rs::model::*;

use std::io::{self, Write};
use tokio;
use zai_rs::client::http::*;

fn extract_text_from_content(v: &serde_json::Value) -> Option<String> {
    // Try the common patterns: string, array of {type: "text", text: ...}, or object with text
    if let Some(s) = v.as_str() {
        return Some(s.to_string());
    }
    if let Some(arr) = v.as_array() {
        let mut buf = String::new();
        for item in arr {
            if let Some(s) = item.get("text").and_then(|t| t.as_str()) {
                buf.push_str(s);
            } else if let Some(s) = item.as_str() {
                buf.push_str(s);
            }
        }
        if !buf.is_empty() {
            return Some(buf);
        }
    }
    if let Some(obj) = v.as_object() {
        if let Some(s) = obj.get("text").and_then(|t| t.as_str()) {
            return Some(s.to_string());
        }
    }
    None
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let model = GLM4_5_airx {};
    let key = std::env::var("ZHIPU_API_KEY").expect("请先在环境变量中设置 ZHIPU_API_KEY");

    println!("可持续对话示例 (输入 exit 或 quit 退出)\n");

    let mut line = String::new();

    // 读取首条用户输入并创建会话
    print!("你> ");
    io::stdout().flush().ok();
    line.clear();
    io::stdin().read_line(&mut line)?;
    let mut user_input = line.trim().to_string();
    if user_input.eq_ignore_ascii_case("exit") || user_input.eq_ignore_ascii_case("quit") {
        return Ok(());
    }

    let mut client = ChatCompletion::new(model, TextMessage::user(&user_input), key)
        .with_temperature(0.7)
        .with_top_p(0.9)
        .with_thinking(ThinkingType::Disabled);

    loop {
        // 发送当前累计的所有消息，并获取 AI 回复
        let resp = client.post().await?;
        let body: ChatCompletionResponse = resp.json().await?;

        // 获取第一条 choice 的文本内容
        let ai_text = body
            .choices()
            .and_then(|cs| cs.get(0))
            .and_then(|c| c.message().content())
            .and_then(|v| extract_text_from_content(v))
            .unwrap_or_else(|| "<empty>".to_string());

        println!("AI> {}\n", ai_text);

        // 将 AI 回复也追加进对话上下文
        client = client.add_messages(TextMessage::assistant(ai_text));

        // 读取下一轮用户输入
        print!("你> ");
        io::stdout().flush().ok();
        line.clear();
        io::stdin().read_line(&mut line)?;
        user_input = line.trim().to_string();
        if user_input.eq_ignore_ascii_case("exit") || user_input.eq_ignore_ascii_case("quit") {
            break;
        }
        if user_input.is_empty() {
            // 空输入则继续读
            continue;
        }

        // 将用户输入追加到对话上下文
        client = client.add_messages(TextMessage::user(&user_input));
    }

    Ok(())
}

