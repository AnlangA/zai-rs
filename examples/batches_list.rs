//! List batches visible to the configured account.

use zai_rs::{
    batches::{BatchListRequest, BatchListResponse},
    client::ZaiClient,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let body: BatchListResponse = BatchListRequest::new().send_via(&client).await?;
    println!("{body:#?}");
    Ok(())
}
