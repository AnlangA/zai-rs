//! Invoke an Agent v1 operation and poll when it is accepted asynchronously.
//!
//! A successful call returns either a completed payload (`id` + `agent_id` +
//! non-empty `choices`) or a pending one (`agent_id` + `async_id`, polled via
//! the typed `AgentAsyncResultRequest` facade).

use zai_rs::{
    ZaiClient, ZaiResult,
    agent::{
        AgentAsyncResultRequest, AgentId, AgentInvokeRequest, AgentInvokeResponse, AgentMessage,
        NonStreaming,
    },
};

#[tokio::main]
async fn main() -> ZaiResult<()> {
    let prompt = std::env::args().nth(1).unwrap_or_else(|| "你好".to_owned());
    let client = ZaiClient::from_env()?;
    let request = AgentInvokeRequest::<NonStreaming>::builder(AgentId::GeneralTranslation)
        .message(AgentMessage::user(prompt))
        .build()?;
    let response = request.send_via(&client).await?;

    match response {
        AgentInvokeResponse::Completed(completed) => {
            println!("{}", serde_json::to_string_pretty(&completed)?);
        },
        AgentInvokeResponse::Pending(pending) => {
            let poll = AgentAsyncResultRequest::new(pending.agent_id, pending.async_id)?;
            let result = poll.send_via(&client).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        },
    }

    Ok(())
}
