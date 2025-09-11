use zai_rs::model::text_tokenizer::{TokenizerMessage, TokenizerModel, TokenizerRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = env_logger::try_init();

    // Read API key
    let key = std::env::var("ZHIPU_API_KEY").expect("Set ZHIPU_API_KEY in your environment");

    // Build messages (minimum 1). Here we send a single user message.
    let messages = vec![TokenizerMessage::User {
        content: "What opportunities and challenges will the Chinese large model industry face in 2025?".into(),
    }];

    // Choose a tokenizer-capable model (default is glm-4-plus)
    let model = TokenizerModel::Glm4Plus;

    // Build request and execute
    let req = TokenizerRequest::new(key, model, messages);
    let resp = req.execute().await?;

    println!("id: {}", resp.id);
    println!("prompt_tokens: {}", resp.usage.prompt_tokens);
    println!("created: {}", resp.created);
    if let Some(rid) = resp.request_id { println!("request_id: {}", rid); }

    Ok(())
}

