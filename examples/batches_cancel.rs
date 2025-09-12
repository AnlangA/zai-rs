use std::fs::File;
use std::io::Write;

use serde_json::json;
use zai_rs::batches::*;
use zai_rs::file::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = env_logger::try_init();
    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    // 1) Prepare a minimal .jsonl with a single chat.completions request
    std::fs::create_dir_all("data")?;
    let path = "data/batch_cancel_demo.jsonl";
    let mut f = File::create(path)?;
    let line = json!({
        "custom_id": "cancel-demo-1",
        "method": "POST",
        "url": "/v4/chat/completions",
        "body": {
            "model": "glm-4",
            "messages": [
                {"role": "system", "content": "You are a helpful assistant."},
                {"role": "user", "content": "Say hello"}
            ]
        }
    });
    writeln!(f, "{}", line.to_string())?;

    // 2) Upload the .jsonl file as purpose=batch
    let upload = FileUploadRequest::new(key.clone(), FilePurpose::Batch, path)
        .with_content_type("application/jsonl");
    let file: FileObject = upload.send().await?;
    let file_id = file.id.ok_or_else(|| anyhow::anyhow!("missing file id"))?;

    // 3) Create a batch task using the uploaded file
    let created: CreateBatchResponse = CreateBatchRequest::new(
        key.clone(),
        file_id,
        BatchEndpoint::ChatCompletions,
    )
    .with_auto_delete_input_file(true)
    .send()
    .await?;

    let batch_id = created
        .id
        .clone()
        .ok_or_else(|| anyhow::anyhow!("create returned no batch id"))?;
    println!("created batch: id={:?} status={:?}", created.id, created.status);

    // 4) Immediately cancel the batch
    let cancelled: CancelBatchResponse = CancelBatchRequest::new(key.clone(), batch_id)
        .send()
        .await?;

    println!("cancelled? id={:?} status={:?}", cancelled.id, cancelled.status);
    Ok(())
}

