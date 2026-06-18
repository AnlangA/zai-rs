use zai_rs::knowledge::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("RUST_LOG").is_some() {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .try_init();
    }
    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    // Args: <document_id>
    let doc_id = std::env::args()
        .nth(1)
        .expect("Usage: knowledge_document_delete <document_id>");

    let req = DocumentDeleteRequest::new(key, doc_id);
    let resp: DocumentDeleteResponse = req.send().await?;

    tracing::trace!(
        "code={:?} message={:?} timestamp={:?}",
        resp.code,
        resp.message,
        resp.timestamp
    );
    Ok(())
}
