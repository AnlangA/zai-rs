//! List images extracted from a knowledge-base document.

use zai_rs::{
    client::ZaiClient,
    knowledge::{DocumentImageListRequest, DocumentImageListResponse},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let doc_id = std::env::args()
        .nth(1)
        .ok_or("usage: knowledge_document_image_list <document-id>")?;

    let client = ZaiClient::from_env()?;
    let req = DocumentImageListRequest::new(doc_id);
    let resp: DocumentImageListResponse = req.send_via(&client).await?;

    println!(
        "code={:?} message={:?} timestamp={:?}",
        resp.code, resp.message, resp.timestamp
    );
    if let Some(images) = resp.data.as_ref().and_then(|data| data.images.as_ref()) {
        for it in images {
            println!("image: text={:?} url={:?}", it.text, it.cos_url);
        }
    }

    Ok(())
}
