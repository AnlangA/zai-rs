use zai_rs::client::v2::ZaiClient;
use zai_rs::knowledge::document_upload_url::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let knowledge_id = std::env::args()
        .nth(1)
        .expect("usage: knowledge_document_upload_url <knowledge_id>");
    let url = std::env::args()
        .nth(2)
        .expect("usage: knowledge_document_upload_url <knowledge_id> <url>");
    let detail = UploadUrlDetail::new(url);
    let body = UploadUrlBody {
        upload_detail: vec![detail],
        knowledge_id: knowledge_id.clone(),
    };
    let resp = DocumentUploadUrlRequest::new(body)
        .send_via(&client)
        .await?;
    println!("{resp:#?}");
    Ok(())
}
