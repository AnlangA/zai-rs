use zai_rs::knowledge::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("RUST_LOG").is_some() {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .try_init();
    }
    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    // Args: <id> <description>
    let id = std::env::args()
        .nth(1)
        .expect("Usage: knowledge_update <id> <description>");
    let description = "修改知识库描述";

    let req = KnowledgeUpdateRequest::new(key, id).with_description(description);
    let resp: KnowledgeUpdateResponse = req.send().await?;
    tracing::trace!(
        "code={:?} message={:?} timestamp={:?}",
        resp.code,
        resp.message,
        resp.timestamp
    );

    Ok(())
}
