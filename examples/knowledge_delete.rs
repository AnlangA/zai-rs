use zai_rs::knowledge::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = env_logger::try_init();
    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    let id = std::env::args()
        .nth(1)
        .expect("Usage: knowledge_delete <id>");

    let resp: KnowledgeDeleteResponse = KnowledgeDeleteRequest::new(key, id).send().await?;
    println!(
        "code={:?} message={:?} timestamp={:?}",
        resp.code, resp.message, resp.timestamp
    );

    Ok(())
}
