use zai_rs::model::text_embedded::{EmbeddingDimensions, EmbeddingInput, EmbeddingModel, EmbeddingRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = env_logger::try_init();

    // Read API key
    let key = std::env::var("ZHIPU_API_KEY").expect("Set ZHIPU_API_KEY in your environment");

    // Build a request: model=embedding-3, single input, optional dimensions
    let model = EmbeddingModel::Embedding3;
    let input = EmbeddingInput::Single("你好，今天天气怎么样.".to_string());

    let req = EmbeddingRequest::new(key, model, input)
        .with_dimensions(EmbeddingDimensions::D256); // embedding-3 supports 256/512/1024/2048

    // Optional: explicit validation (send() will validate automatically)
    if let Err(e) = req.validate() {
        eprintln!("Validation warning: {:?}", e);
    }

    // Send and print summary
    let resp = req.send().await?;

    println!("model: {}", resp.model);
    println!("object: {:?}", resp.object);
    println!("items: {}", resp.data.len());

    for item in &resp.data {
        println!("- index={} object={:?} dims={}", item.index, item.object, item.embedding.len());
        // Print first few numbers for brevity
        let preview: Vec<String> = item
            .embedding
            .iter()
            .take(8)
            .map(|x| format!("{:.6}", x))
            .collect();
        println!("  preview: [{}]{}", preview.join(", "), if item.embedding.len() > 8 { " ..." } else { "" });
    }

    println!(
        "usage: prompt_tokens={} completion_tokens={} total_tokens={}",
        resp.usage.prompt_tokens, resp.usage.completion_tokens, resp.usage.total_tokens
    );

    Ok(())
}

