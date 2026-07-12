//! Upload one or more local documents to a knowledge base.

use zai_rs::{
    client::ZaiClient,
    knowledge::{DocumentUploadRequest, DocumentUploadResponse},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let knowledge_id = args
        .next()
        .ok_or("usage: knowledge_document_upload_file <knowledge-id> <file> [file ...]")?;
    let first_file = args
        .next()
        .ok_or("usage: knowledge_document_upload_file <knowledge-id> <file> [file ...]")?;

    let mut request = DocumentUploadRequest::new(knowledge_id).add_file_path(first_file);
    for file in args {
        request = request.add_file_path(file);
    }

    let client = ZaiClient::from_env()?;
    let response: DocumentUploadResponse = request.send_via(&client).await?;
    println!("{response:#?}");

    Ok(())
}
