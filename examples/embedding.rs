use zai_rs::model::text_embedded::{
    EmbeddingDimensions, EmbeddingInput, EmbeddingModel, EmbeddingRequest,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("RUST_LOG").is_some() {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .try_init();
    }

    // Read API key
    let key = std::env::var("ZHIPU_API_KEY").expect("Set ZHIPU_API_KEY in your environment");

    // Build a request: model=embedding-3, single input, optional dimensions
    let model = EmbeddingModel::Embedding3;
    let input = EmbeddingInput::Single("你好，今天天气怎么样.".to_string());

    let req = EmbeddingRequest::new(key, model, input).with_dimensions(EmbeddingDimensions::D256); // embedding-3 supports 256/512/1024/2048

    // Optional: explicit validation (send() will validate automatically)
    if let Err(e) = req.validate() {
        tracing::warn!(error = ?e, "Validation warning");
    }

    // Send and print summary
    let resp = req.send().await?;

    tracing::trace!("model: {}", resp.model);
    tracing::trace!("object: {:?}", resp.object);
    tracing::trace!("items: {}", resp.data.len());

    for item in &resp.data {
        tracing::trace!(
            "- index={} object={:?} dims={}",
            item.index,
            item.object,
            item.embedding.len()
        );
        // Print first few numbers for brevity
        let preview: Vec<String> = item
            .embedding
            .iter()
            .take(8)
            .map(|x| format!("{:.6}", x))
            .collect();
        tracing::trace!(
            "  preview: [{}]{}",
            preview.join(", "),
            if item.embedding.len() > 8 { " ..." } else { "" }
        );
    }

    tracing::trace!(
        "usage: prompt_tokens={} completion_tokens={} total_tokens={}",
        resp.usage.prompt_tokens,
        resp.usage.completion_tokens,
        resp.usage.total_tokens
    );

    Ok(())
}
