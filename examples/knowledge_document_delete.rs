use zai_rs::knowledge::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = env_logger::try_init();
    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    // Args: <document_id>
    let doc_id = std::env::args().nth(1).expect("Usage: knowledge_document_delete <document_id>");

    let req = DocumentDeleteRequest::new(key, doc_id);
    let resp: DocumentDeleteResponse = req.send().await?;

    println!("code={:?} message={:?} timestamp={:?}", resp.code, resp.message, resp.timestamp);
    Ok(())
}

