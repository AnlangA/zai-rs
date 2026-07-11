use zai_rs::client::ZaiClient;
use zai_rs::knowledge::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;

    // Args: <document_id>
    let doc_id = std::env::args()
        .nth(1)
        .expect("Usage: knowledge_document_image_list <document_id>");

    let req = DocumentImageListRequest::new(doc_id);
    let resp: DocumentImageListResponse = req.send_via(&client).await?;

    println!(
        "code={:?} message={:?} timestamp={:?}",
        resp.code, resp.message, resp.timestamp
    );
    if let Some(data) = &resp.data
        && let Some(images) = &data.images
    {
        for it in images.iter() {
            println!("image: text={:?} url={:?}", it.text, it.cos_url);
        }
    }

    Ok(())
}
