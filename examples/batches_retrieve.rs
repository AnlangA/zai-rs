use std::time::Duration;

use zai_rs::{batches::*, file::*};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("RUST_LOG").is_some() {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .try_init();
    }
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
        tracing::trace!("poll[{}]: status={}", attempt, status);
        if status == "completed" || status == "failed" || attempt >= max_attempts {
            break batch;
        }
        attempt += 1;
        tokio::time::sleep(Duration::from_secs(2)).await;
    };

    tracing::trace!(
        "batch id={:?} status={:?} endpoint={:?}",
        final_batch.id,
        final_batch.status,
        final_batch.endpoint
    );
    tracing::trace!(
        "output_file_id={:?} error_file_id={:?}",
        final_batch.output_file_id,
        final_batch.error_file_id
    );

    std::fs::create_dir_all("data")?;

    // Download output_file_id if present
    if let Some(out_id) = final_batch.output_file_id.clone() {
        FileContentRequest::new(key.clone(), out_id)
            .send_to("data/batch_output.jsonl")
            .await?;
        tracing::trace!("saved: data/batch_output.jsonl");
    } else {
        tracing::trace!("no output_file_id yet");
    }

    // Download error_file_id if present
    if let Some(err_id) = final_batch.error_file_id.clone() {
        FileContentRequest::new(key.clone(), err_id)
            .send_to("data/batch_errors.jsonl")
            .await?;
        tracing::trace!("saved: data/batch_errors.jsonl");
    } else {
        tracing::trace!("no error_file_id");
    }

    Ok(())
}
