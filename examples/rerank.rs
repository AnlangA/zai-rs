use zai_rs::model::text_rerank::RerankRequest;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = env_logger::try_init();

    // Read API key
    let key = std::env::var("ZHIPU_API_KEY").expect("Set ZHIPU_API_KEY in your environment");

    // Query and candidate documents
    let query = "要查询的文本";
    let documents = vec![
        "要查询的文本".to_string(),
        "这个文本分数低".to_string(),
    ];

    // Build request
    let req = RerankRequest::new(key, query, documents)
        .with_top_n(4)
        .with_return_documents(true)
        .with_return_raw_scores(true);

    // Optional runtime validation (send() will validate automatically)
    if let Err(e) = req.validate() {
        eprintln!("Validation warning: {e}");
    }

    // Send
    let resp = req.send().await?;

    println!("created: {}", resp.created);
    println!("id: {}", resp.id);
    if let Some(rid) = &resp.request_id { println!("request_id: {}", rid); }

    println!("results: {}", resp.results.len());
    for r in &resp.results {
        println!("- index={} score={:.6}", r.index, r.relevance_score);
        if let Some(doc) = &r.document { println!("  doc: {}", doc); }
    }

    println!(
        "usage: prompt_tokens={} total_tokens={}",
        resp.usage.prompt_tokens, resp.usage.total_tokens
    );

    Ok(())
}

