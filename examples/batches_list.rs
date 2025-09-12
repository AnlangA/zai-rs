use zai_rs::batches::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Set your API key in env: ZHIPU_API_KEY
    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    // Build query (optional)
    let query = BatchesListQuery::new()
        // .with_after("cursor_or_id_here")
        .with_limit(20);

    // Send request: GET /paas/v4/batches and parse JSON
    let client = BatchesListRequest::new(key).with_query(query);
    let body: BatchesListResponse = client.send().await?;

    println!("object: {:?}", body.object);
    println!("has_more: {:?}", body.has_more);

    if let Some(items) = body.data.as_ref() {
        println!("batches: {}", items.len());
        for (i, b) in items.iter().enumerate() {
            println!(
                "#{} id={:?} status={:?} endpoint={:?}",
                i + 1,
                b.id,
                b.status,
                b.endpoint
            );
        }
    }

    Ok(())
}
