//! Add a remote document URL to a knowledge base.

use zai_rs::{
    client::ZaiClient,
    knowledge::{DocumentUrlUploadBody, DocumentUrlUploadRequest},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let knowledge_id = args
        .next()
        .ok_or("usage: knowledge_document_upload_url <knowledge-id> <url>")?;
    let url = args
        .next()
        .ok_or("usage: knowledge_document_upload_url <knowledge-id> <url>")?;
    let body = DocumentUrlUploadBody::new(knowledge_id).add_url(url);

    let client = ZaiClient::from_env()?;
    let resp = DocumentUrlUploadRequest::new(body)
        .send_via(&client)
        .await?;
    println!("{resp:#?}");
    Ok(())
}
