use zai_rs::client::ZaiClient;
use zai_rs::model::text_tokenizer::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let request = TokenizerRequest::new(
        TokenizerModel::default(),
        vec![TokenizerMessage::User {
            content: "Hello world".to_string(),
        }],
    );
    let resp = request.send_via(&client).await?;
    println!("{resp:#?}");
    Ok(())
}
