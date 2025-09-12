
use std::time::Duration;

use zai_rs::batches::*;

use zai_rs::file::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = env_logger::try_init();
    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    // Get batch_id from CLI arg or BATCH_ID env; otherwise show usage and exit gracefully
    let batch_id = "batch_1966317613131636736";

    // Poll until completed/failed (max ~2 minutes)
    let mut attempt = 0u32;
    let max_attempts = 60u32;
    let final_batch = loop {
        let req = BatchesRetrieveRequest::new(key.clone(), batch_id.clone());
        let batch: BatchesRetrieveResponse = req.send().await?;
        let status = batch.status.clone().unwrap_or_else(|| "unknown".to_string());
        println!("poll[{}]: status={}", attempt, status);
        if status == "completed" || status == "failed" || attempt >= max_attempts {
            break batch;
        }
        attempt += 1;
        tokio::time::sleep(Duration::from_secs(2)).await;
    };

    println!("batch id={:?} status={:?} endpoint={:?}", final_batch.id, final_batch.status, final_batch.endpoint);
    println!("output_file_id={:?} error_file_id={:?}", final_batch.output_file_id, final_batch.error_file_id);

    std::fs::create_dir_all("data")?;

    // Download output_file_id if present
    if let Some(out_id) = final_batch.output_file_id.clone() {
        FileContentRequest::new(key.clone(), out_id).send_to("data/batch_output.jsonl").await?;
        println!("saved: data/batch_output.jsonl");
    } else {
        println!("no output_file_id yet");
    }

    // Download error_file_id if present
    if let Some(err_id) = final_batch.error_file_id.clone() {
        FileContentRequest::new(key.clone(), err_id).send_to("data/batch_errors.jsonl").await?;
        println!("saved: data/batch_errors.jsonl");
    } else {
        println!("no error_file_id");
    }

    Ok(())
}

