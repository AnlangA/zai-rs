use zai_rs::knowledge::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = env_logger::try_init();
    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    // Args: <document_id>
    let doc_id = std::env::args().nth(1).expect("Usage: knowledge_document_detail <document_id>");

    let req = DocumentRetrieveRequest::new(key, doc_id);
    let resp: DocumentDetailResponse = req.send().await?;

    println!("code={:?} message={:?} timestamp={:?}", resp.code, resp.message, resp.timestamp);
    if let Some(doc) = &resp.data {
        println!("id={:?} name={:?} type={:?} words={:?} bytes={:?}", doc.id, doc.name, doc.knowledge_type, doc.word_num, doc.length);
        if let Some(f) = &doc.fail_info {
            println!("fail: code={:?} msg={:?}", f.embedding_code, f.embedding_msg);
        }
    }

    Ok(())
}

