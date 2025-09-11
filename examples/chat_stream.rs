use std::io::Write;
use std::sync::Arc;
use tokio;
use tokio::sync::Mutex;
use zai_rs::model::*; // includes ChatStreamResponse re-export

// Stream chat completions as server-sent events (SSE) and print each data chunk as it arrives.
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
                    .get(0)
                    .and_then(|c| c.delta.as_ref())
                    .and_then(|d| d.content.as_deref())
                {
                    print!("{}", content);
                    let _ = std::io::stdout().flush();
                }

                if let Some(reason) = chunk.choices.get(0).and_then(|c| c.finish_reason.as_ref()) {
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
