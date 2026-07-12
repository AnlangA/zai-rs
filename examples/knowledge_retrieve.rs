//! Retrieve a knowledge base by ID.

use zai_rs::{client::ZaiClient, knowledge::KnowledgeGetRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let id = std::env::args()
        .nth(1)
        .ok_or("usage: knowledge_retrieve <knowledge-id>")?;

    let client = ZaiClient::from_env()?;
    let resp = KnowledgeGetRequest::new(id).send_via(&client).await?;
    println!("{resp:#?}");
    Ok(())
}
