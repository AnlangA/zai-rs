//! List knowledge bases visible to the configured account.

use zai_rs::{client::ZaiClient, knowledge::KnowledgeListRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let resp = KnowledgeListRequest::new().send_via(&client).await?;
    println!("{resp:#?}");
    Ok(())
}
