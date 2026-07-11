//! # Tool-calling Example (non-streaming)
//!
//! P05 note: the SSE streaming path is rebuilt in P08.

use zai_rs::client::ZaiClient;
use zai_rs::model::{chat_base_response::ChatCompletionResponse, *};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let model = GLM4_5 {};

    let request = ChatCompletion::new(model, TextMessage::user("What is 2+2?"));
    let body: ChatCompletionResponse = request.send_via_coding_plan(&client).await?;
    println!("{body:#?}");
    Ok(())
}
