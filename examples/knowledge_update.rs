//! Update the description of an existing knowledge base.

use zai_rs::{
    client::ZaiClient,
    knowledge::{KnowledgeUpdateRequest, KnowledgeUpdateResponse},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let id = args
        .next()
        .ok_or("usage: knowledge_update <knowledge-id> <description>")?;
    let description = args
        .next()
        .ok_or("usage: knowledge_update <knowledge-id> <description>")?;

    let client = ZaiClient::from_env()?;
    let resp: KnowledgeUpdateResponse = KnowledgeUpdateRequest::new(id)
        .with_description(description)
        .send_via(&client)
        .await?;
    println!("{resp:#?}");
    Ok(())
}
