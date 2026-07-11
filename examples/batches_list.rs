use zai_rs::batches::*;
use zai_rs::client::ZaiClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let body: BatchesListResponse = BatchesListRequest::new().send_via(&client).await?;
    println!("{body:#?}");
    Ok(())
}
