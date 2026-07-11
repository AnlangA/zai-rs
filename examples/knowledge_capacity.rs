use zai_rs::client::v2::ZaiClient;
use zai_rs::knowledge::capacity::KnowledgeCapacityRequest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let resp = KnowledgeCapacityRequest::new().send_via(&client).await?;
    println!("{resp:#?}");
    Ok(())
}
