use zai_rs::client::ZaiClient;
use zai_rs::knowledge::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;

    // Args: name [embedding=2|3new] [background] [icon]
    let name = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "my-knowledge".to_string());

    // Simple arg mapping for demo purposes
    let emb = match std::env::args().nth(2).as_deref() {
        Some("3new") => EmbeddingId::Embedding3New,
        _ => EmbeddingId::Embedding2,
    };
    let bg = match std::env::args().nth(3).as_deref() {
        Some("red") => Some(BackgroundColor::Red),
        Some("orange") => Some(BackgroundColor::Orange),
        Some("purple") => Some(BackgroundColor::Purple),
        Some("sky") => Some(BackgroundColor::Sky),
        Some("green") => Some(BackgroundColor::Green),
        Some("yellow") => Some(BackgroundColor::Yellow),
        Some("blue") => Some(BackgroundColor::Blue),
        _ => None,
    };
    let icon = match std::env::args().nth(4).as_deref() {
        Some("book") => Some(KnowledgeIcon::Book),
        Some("seal") => Some(KnowledgeIcon::Seal),
        Some("wrench") => Some(KnowledgeIcon::Wrench),
        Some("tag") => Some(KnowledgeIcon::Tag),
        Some("horn") => Some(KnowledgeIcon::Horn),
        Some("house") => Some(KnowledgeIcon::House),
        Some("question") => Some(KnowledgeIcon::Question),
        _ => None,
    };

    let mut req =
        CreateKnowledgeRequest::new(emb, name).with_description("Created by zai-rs example");
    if let Some(c) = bg {
        req = req.with_background(c);
    }
    if let Some(i) = icon {
        req = req.with_icon(i);
    }

    let resp: CreateKnowledgeResponse = req.send_via(&client).await?;
    println!("code={:?} message={:?}", resp.code, resp.message);
    if let Some(data) = resp.data {
        println!("created id={:?}", data.id);
    }

    Ok(())
}
