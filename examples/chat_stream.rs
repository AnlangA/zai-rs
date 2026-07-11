//! # Chat Example (non-streaming)
//!
//! P05 note: the full SSE streaming path (`stream_for_each`) is rebuilt in P08.
//! Until then this example uses the non-streaming `send_via` path.

use zai_rs::client::v2::ZaiClient;
use zai_rs::model::{chat_base_response::ChatCompletionResponse, *};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let model = GLM4_5 {};

    let request = ChatCompletion::new(model, TextMessage::user("Hello,黑神话悟空讲了什么叙事"));
    let body: ChatCompletionResponse = request.send_via(&client).await?;

    if let Some(content) = body
        .choices()
        .and_then(|cs| cs.first())
        .and_then(|c| c.message().content())
    {
        println!("{content}");
    }
    Ok(())
}
