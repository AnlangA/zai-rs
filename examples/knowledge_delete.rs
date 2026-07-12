//! Delete a knowledge base by ID.

use zai_rs::{client::ZaiClient, knowledge::KnowledgeDeleteRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let id = std::env::args()
        .nth(1)
        .ok_or("usage: knowledge_delete <knowledge-id>")?;

    let client = ZaiClient::from_env()?;
    let resp = KnowledgeDeleteRequest::new(id).send_via(&client).await?;
    println!("{resp:#?}");
    Ok(())
}
