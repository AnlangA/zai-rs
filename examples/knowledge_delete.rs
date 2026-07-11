use zai_rs::client::v2::ZaiClient;
use zai_rs::knowledge::delete::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let id = std::env::args()
        .nth(1)
        .expect("usage: knowledge_delete <id>");
    let resp = KnowledgeDeleteRequest::new(id).send_via(&client).await?;
    println!("{resp:#?}");
    Ok(())
}
