use zai_rs::knowledge::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = env_logger::try_init();
    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    // Args: <id> <description>
    let id = std::env::args()
        .nth(1)
        .expect("Usage: knowledge_update <id> <description>");
    let description = "修改知识库描述";

    let req = KnowledgeUpdateRequest::new(key, id).with_description(description);
    let resp: KnowledgeUpdateResponse = req.send().await?;
    println!(
        "code={:?} message={:?} timestamp={:?}",
        resp.code, resp.message, resp.timestamp
    );

    Ok(())
}
