//! List documents in a knowledge base, optionally filtering by a word.

use zai_rs::{
    client::ZaiClient,
    knowledge::{DocumentListRequest, DocumentListResponse},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let knowledge_id = args
        .next()
        .ok_or("usage: knowledge_document_list <knowledge-id> [word]")?;
    let request = match args.next() {
        Some(word) => DocumentListRequest::new(knowledge_id).with_word(word),
        None => DocumentListRequest::new(knowledge_id),
    };

    let client = ZaiClient::from_env()?;
    let response: DocumentListResponse = request.send_via(&client).await?;
    println!("{response:#?}");

    Ok(())
}
