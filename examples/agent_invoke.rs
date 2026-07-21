//! Build the frozen Agent v1 invocation request body.
//!
//! Agent v1 currently ships wire contracts only: `zai_rs::agent` intentionally
//! contains no network facade and no `send_via` path, so there is no typed
//! client call to demonstrate. This example constructs the exact request body
//! for `POST /v1/agents` and prints it; send that JSON with any HTTP client:
//!
//! ```text
//! POST https://open.bigmodel.cn/api/v1/agents
//! Authorization: Bearer $ZHIPU_API_KEY
//! Content-Type: application/json
//! ```
//!
//! A successful call returns either a completed payload (`id` + `agent_id` +
//! non-empty `choices`) or a pending one (`agent_id` + `async_id`, polled via
//! `POST /v1/agents/async-result` with an `AgentAsyncResultRequest` body);
//! both decode into `AgentInvokeResponse`. For a comparable conversational
//! API with a typed `send_via` path, see `examples/assistant.rs`.

use zai_rs::{
    ZaiResult,
    agent::{AgentAsyncResultRequest, AgentId, AgentInvokeRequest, AgentMessage, NonStreaming},
};

fn main() -> ZaiResult<()> {
    let prompt = std::env::args().nth(1).unwrap_or_else(|| "你好".to_owned());

    let request = AgentInvokeRequest::<NonStreaming>::builder(AgentId::GeneralTranslation)
        .message(AgentMessage::user(prompt))
        .build()?;
    println!("POST /v1/agents");
    println!("{}", serde_json::to_string_pretty(&request)?);

    // When the invocation returns a pending payload, poll for the result with
    // the two identifiers it contains.
    let poll = AgentAsyncResultRequest::new(
        AgentId::GeneralTranslation.as_str(),
        "<async_id from the pending response>",
    )?;
    println!("POST /v1/agents/async-result");
    println!("{}", serde_json::to_string_pretty(&poll)?);

    Ok(())
}
