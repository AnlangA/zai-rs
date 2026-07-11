use zai_rs::client::ZaiClient;
use zai_rs::knowledge::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let doc_id = std::env::args()
        .nth(1)
        .expect("usage: knowledge_document_detail <doc_id>");
    let resp = DocumentRetrieveRequest::new(doc_id)
        .send_via(&client)
        .await?;
    println!("{resp:#?}");
    Ok(())
}
