use zai_rs::batches::*;
use zai_rs::client::v2::ZaiClient;
use zai_rs::file::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let path = std::env::temp_dir().join("zai_batch_cancel.jsonl");
    let path_str = path.to_str().unwrap();
    std::fs::write(
        &path,
        r#"{"custom_id":"d","method":"POST","url":"/v4/chat/completions","body":{"model":"glm-4","messages":[{"role":"user","content":"hi"}]}}"#,
    )?;
    let file: FileObject = FileUploadRequest::new(FilePurpose::Batch, path_str)
        .send_via(&client)
        .await?;
    let file_id = file.id.ok_or("no id")?;
    let created: CreateBatchResponse =
        CreateBatchRequest::new(file_id, BatchEndpoint::ChatCompletions)
            .send_via(&client)
            .await?;
    let batch_id = created.id.ok_or("no batch id")?;
    let cancelled: CancelBatchResponse =
        CancelBatchRequest::new(batch_id).send_via(&client).await?;
    println!("{cancelled:#?}");
    Ok(())
}
