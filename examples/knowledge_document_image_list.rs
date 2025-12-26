use zai_rs::knowledge::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = env_logger::try_init();
    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    // Args: <document_id>
    let doc_id = std::env::args()
        .nth(1)
        .expect("Usage: knowledge_document_image_list <document_id>");

    let req = DocumentImageListRequest::new(key, doc_id);
    let resp: DocumentImageListResponse = req.send().await?;

    println!(
        "code={:?} message={:?} timestamp={:?}",
        resp.code, resp.message, resp.timestamp
    );
    if let Some(data) = &resp.data {
        if let Some(images) = &data.images {
            for it in images.iter() {
                println!("image: text={:?} url={:?}", it.text, it.cos_url);
            }
        }
    }

    Ok(())
}
