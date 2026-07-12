//! Retrieve one batch; use `files_content` to download its output file.

use zai_rs::{
    batches::{BatchGetRequest, BatchGetResponse},
    client::ZaiClient,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let batch_id = std::env::args()
        .nth(1)
        .ok_or("usage: batches_retrieve <batch-id>")?;

    let client = ZaiClient::from_env()?;
    let batch: BatchGetResponse = BatchGetRequest::new(batch_id).send_via(&client).await?;
    println!("{batch:#?}");
    Ok(())
}
