//! Invoke an assistant with one user message and print the response.

use zai_rs::{
    ZaiResult,
    client::ZaiClient,
    services::assistants::{
        AssistantId, AssistantInvokeRequest, AssistantInvokeResponse, AssistantMessage,
    },
};

#[tokio::main]
async fn main() -> ZaiResult<()> {
    let prompt = std::env::args().nth(1).unwrap_or_else(|| "你好".to_owned());

    let client = ZaiClient::from_env()?;
    let response: AssistantInvokeResponse =
        AssistantInvokeRequest::new(AssistantId::ChatGlm, vec![AssistantMessage::user(prompt)])
            .send_via(&client)
            .await?;
    println!("{response:#?}");

    Ok(())
}
