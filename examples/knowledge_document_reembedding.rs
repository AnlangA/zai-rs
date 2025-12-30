use std::collections::BTreeMap;

use zai_rs::knowledge::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = env_logger::try_init();
    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    // Args: <document_id> [callback_url]
    let doc_id = std::env::args()
        .nth(1)
        .expect("Usage: knowledge_document_reembedding <document_id> [callback_url]");
    let cb = std::env::args().nth(2);

    let mut req = DocumentReembeddingRequest::new(key, doc_id);
    if let Some(url) = cb {
        req = req.with_callback_url(url);
        let mut hdr = BTreeMap::new();
        hdr.insert("X-Trace".to_string(), "zai-rs-example".to_string());
        req = req.with_callback_header(hdr);
    }

    let resp: DocumentReembeddingResponse = req.send().await?;
    println!(
        "code={:?} message={:?} timestamp={:?}",
        resp.code, resp.message, resp.timestamp
    );

    Ok(())
}
