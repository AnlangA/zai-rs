use zai_rs::client::ZaiClient;
use zai_rs::knowledge::list::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let resp = KnowledgeListRequest::new().send_via(&client).await?;
    println!("{resp:#?}");
    Ok(())
}
