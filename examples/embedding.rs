use zai_rs::client::ZaiClient;
use zai_rs::model::text_embedded::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let request = EmbeddingRequest::new(
        EmbeddingModel::Embedding2,
        EmbeddingInput::Single("Hello world".to_string()),
    );
    let resp = request.send_via(&client).await?;
    println!("{resp:#?}");
    Ok(())
}
