use zai_rs::batches::*;
use zai_rs::client::v2::ZaiClient;
use zai_rs::file::FileContentRequest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let batch_id = std::env::args()
        .nth(1)
        .expect("usage: batches_retrieve <batch_id>");
    let batch: BatchesRetrieveResponse = BatchesRetrieveRequest::new(batch_id)
        .send_via(&client)
        .await?;
    println!("{batch:#?}");
    if let Some(out_id) = batch.output_file_id {
        let p = std::env::temp_dir().join("zai_batch_output.jsonl");
        FileContentRequest::new(out_id)
            .send_to_via(&client, p.to_str().unwrap())
            .await?;
    }
    Ok(())
}
