//! Re-embed a document, optionally notifying a callback URL on completion.

use zai_rs::{
    client::ZaiClient,
    knowledge::{DocumentReembedRequest, DocumentReembedResponse},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let doc_id = args
        .next()
        .ok_or("usage: knowledge_document_reembedding <document-id> [callback-url]")?;

    let request = match args.next() {
        Some(url) => DocumentReembedRequest::new(doc_id).with_callback_url(url),
        None => DocumentReembedRequest::new(doc_id),
    };

    let client = ZaiClient::from_env()?;
    let response: DocumentReembedResponse = request.send_via(&client).await?;
    println!("{response:#?}");

    Ok(())
}
