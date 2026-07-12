//! Create a knowledge base with the current embedding model.

use zai_rs::{
    client::ZaiClient,
    knowledge::{EmbeddingId, KnowledgeCreateRequest, KnowledgeCreateResponse},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let name = std::env::args()
        .nth(1)
        .ok_or("usage: knowledge_create <name>")?;

    let client = ZaiClient::from_env()?;
    let response: KnowledgeCreateResponse =
        KnowledgeCreateRequest::new(EmbeddingId::Embedding3New, name)
            .with_description("Created with zai-rs")
            .send_via(&client)
            .await?;
    println!("{response:#?}");

    Ok(())
}
