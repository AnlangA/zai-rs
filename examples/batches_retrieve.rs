use std::time::Duration;

use zai_rs::{batches::*, file::*};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    // Get batch_id from CLI arg or BATCH_ID env; otherwise show usage and exit
    // gracefully
    let batch_id = "batch_1966317613131636736";

    // Poll until completed/failed (max ~2 minutes)
    let mut attempt = 0u32;
    let max_attempts = 60u32;
    let final_batch = loop {
        let req = BatchesRetrieveRequest::new(key.clone(), batch_id);
        let batch: BatchesRetrieveResponse = req.send().await?;
        let status = batch
            .status
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        println!("poll[{attempt}]: status={status}");
        if status == "completed" || status == "failed" || attempt >= max_attempts {
            break batch;
        }
        attempt += 1;
        tokio::time::sleep(Duration::from_secs(2)).await;
    };

    println!(
        "batch id={:?} status={:?} endpoint={:?}",
        final_batch.id, final_batch.status, final_batch.endpoint
    );
    println!(
        "output_file_id={:?} error_file_id={:?}",
        final_batch.output_file_id, final_batch.error_file_id
    );

    // Download into a temp location (no longer under the removed `data/` dir).
    let out_path = std::env::temp_dir().join("zai_batch_output.jsonl");
    let out_path_str = out_path.to_str().expect("temp path is valid utf-8");
    let err_path = std::env::temp_dir().join("zai_batch_errors.jsonl");
    let err_path_str = err_path.to_str().expect("temp path is valid utf-8");

    // Download output_file_id if present
    if let Some(out_id) = final_batch.output_file_id.clone() {
        FileContentRequest::new(key.clone(), out_id)
            .send_to(out_path_str)
            .await?;
        println!("saved: {out_path_str}");
    } else {
        println!("no output_file_id yet");
    }

    // Download error_file_id if present
    if let Some(err_id) = final_batch.error_file_id.clone() {
        FileContentRequest::new(key.clone(), err_id)
            .send_to(err_path_str)
            .await?;
        println!("saved: {err_path_str}");
    } else {
        println!("no error_file_id");
    }

    Ok(())
}
