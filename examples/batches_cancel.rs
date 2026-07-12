//! Cancel an existing batch without creating unrelated resources first.

use zai_rs::{
    batches::{BatchCancelRequest, BatchCancelResponse},
    client::ZaiClient,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let batch_id = std::env::args()
        .nth(1)
        .ok_or("usage: batches_cancel <batch-id>")?;

    let client = ZaiClient::from_env()?;
    let cancelled: BatchCancelResponse =
        BatchCancelRequest::new(batch_id).send_via(&client).await?;
    println!("{cancelled:#?}");
    Ok(())
}
