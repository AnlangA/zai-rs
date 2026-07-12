//! Create a batch from a JSONL file previously uploaded with purpose `batch`.

use zai_rs::{
    batches::{BatchCreateRequest, BatchCreateResponse, BatchEndpoint},
    client::ZaiClient,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file_id = std::env::args()
        .nth(1)
        .ok_or("usage: batches_create <input-file-id>")?;

    let client = ZaiClient::from_env()?;
    let batch: BatchCreateResponse =
        BatchCreateRequest::new(file_id, BatchEndpoint::ChatCompletions)
            .send_via(&client)
            .await?;
    println!("{batch:#?}");
    Ok(())
}
