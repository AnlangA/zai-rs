use zai_rs::knowledge::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = env_logger::try_init();
    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    let id = std::env::args().nth(1).expect("Usage: knowledge_retrieve <id>");

    let req = KnowledgeRetrieveRequest::new(key, id);
    let resp: KnowledgeRetrieveResponse = req.send().await?;

    println!("code={:?} message={:?} timestamp={:?}", resp.code, resp.message, resp.timestamp);
    if let Some(item) = &resp.data {
        println!(
            "id={:?} name={:?} emb={:?} docs={:?} length={:?} words={:?}",
            item.id, item.name, item.embedding_id, item.document_size, item.length, item.word_num
        );
    }
    Ok(())
}

