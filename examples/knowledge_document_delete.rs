//! Delete a document from its knowledge base.

use zai_rs::{client::ZaiClient, knowledge::DocumentDeleteRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let doc_id = std::env::args()
        .nth(1)
        .ok_or("usage: knowledge_document_delete <document-id>")?;

    let client = ZaiClient::from_env()?;
    let resp = DocumentDeleteRequest::new(doc_id).send_via(&client).await?;
    println!("{resp:#?}");
    Ok(())
}
