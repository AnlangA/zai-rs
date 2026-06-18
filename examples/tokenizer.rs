use zai_rs::model::text_tokenizer::{TokenizerMessage, TokenizerModel, TokenizerRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("RUST_LOG").is_some() {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .try_init();
    }

    // Read API key
    let key = std::env::var("ZHIPU_API_KEY").expect("Set ZHIPU_API_KEY in your environment");

    // Build messages (minimum 1). Here we send a single user message.
    let messages = vec![TokenizerMessage::User {
        content:
            "What opportunities and challenges will the Chinese large model industry face in 2025?"
                .into(),
    }];

    // Choose a tokenizer-capable model (default is glm-4-plus)
    let model = TokenizerModel::Glm4Plus;

    // Build request and send
    let req = TokenizerRequest::new(key, model, messages);
    let resp = req.send().await?;

    tracing::trace!("id: {}", resp.id);
    tracing::trace!("prompt_tokens: {}", resp.usage.prompt_tokens);
    tracing::trace!("created: {}", resp.created);
    if let Some(rid) = resp.request_id {
        tracing::trace!("request_id: {}", rid);
    }

    Ok(())
}
