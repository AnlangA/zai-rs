//! # Chat Streaming Example
//!
//! This example demonstrates how to use the ZAI-RS SDK for streaming chat
//! completions using Server-Sent Events (SSE) with the Zhipu AI API.
//!
//! ## Features Demonstrated
//!
//! - Streaming chat completions with real-time responses
//! - Server-Sent Events (SSE) processing
//! - Asynchronous response handling
//! - Token-by-token output printing
//! - Finish reason tracking
//!
//! ## Prerequisites
//!
//! Set the `ZHIPU_API_KEY` environment variable with your API key:
//! ```bash
//! export ZHIPU_API_KEY="your-api-key-here"
//! ```
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run --example chat_stream
//! ```
//!
//! ## How It Works
//!
//! 1. Creates a streaming chat request with the GLM-4.5 model
//! 2. Enables streaming mode to receive responses as they are generated
//! 3. Processes each SSE chunk as it arrives from the server
//! 4. Prints response content token-by-token for real-time feedback
//! 5. Tracks and displays the finish reason when the response completes
//!
//! ## Output
//!
//! The example will print the AI's response character by character as it's
//! generated, followed by the finish reason (e.g., "stop", "length", etc.).

use std::{io::Write, sync::Arc};

use tokio::sync::Mutex;
use zai_rs::model::*; // includes ChatStreamResponse re-export

/// Stream chat completions as server-sent events (SSE) and print each data
/// chunk as it arrives.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1) Read API key from environment
    let key = std::env::var("ZHIPU_API_KEY").expect("Set ZHIPU_API_KEY in your environment");

    // Build a streaming chat request
    let model = GLM4_5 {};
    let mut client = ChatCompletion::new(
        model,
        TextMessage::user("Hello,黑神话悟空讲了什么叙事"),
        key,
    )
    .enable_stream();

    let finish = Arc::new(Mutex::new(None::<String>));
    let finish2 = finish.clone();
    client
        .stream_for_each(move |chunk: ChatStreamResponse| {
            let finish = finish2.clone();
            async move {
                if let Some(content) = chunk
                    .choices
                    .first()
                    .and_then(|c| c.delta.as_ref())
                    .and_then(|d| d.content.as_deref())
                {
                    print!("{}", content);
                    let _ = std::io::stdout().flush();
                }

                if let Some(reason) = chunk.choices.first().and_then(|c| c.finish_reason.as_ref()) {
                    let mut g = finish.lock().await;
                    *g = Some(reason.clone());
                }
                Ok(())
            }
        })
        .await?;

    let last_finish_reason = finish.lock().await.clone();
    println!();
    println!(
        "{}",
        last_finish_reason
            .as_deref()
            .unwrap_or("finish_reason: <none>")
    );
    Ok(())
}
