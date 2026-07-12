use zai_rs::client::ZaiClient;
use zai_rs::knowledge::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let id = std::env::args()
        .nth(1)
        .expect("usage: knowledge_update <id>");
    let description = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "updated".to_string());
    let resp: KnowledgeUpdateResponse = KnowledgeUpdateRequest::new(id)
        .with_description(description)
        .send_via(&client)
        .await?;
    println!("{resp:#?}");
    Ok(())
}
