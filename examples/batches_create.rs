use zai_rs::batches::*;
use zai_rs::client::v2::ZaiClient;
use zai_rs::file::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let path = std::env::temp_dir().join("zai_batch_demo.jsonl");
    let path_str = path.to_str().unwrap();
    std::fs::write(
        &path,
        r#"{"custom_id":"r1","method":"POST","url":"/v4/chat/completions","body":{"model":"glm-4","messages":[{"role":"user","content":"hi"}]}}"#,
    )?;
    let file: FileObject = FileUploadRequest::new(FilePurpose::Batch, path_str)
        .with_content_type("application/jsonl")
        .send_via(&client)
        .await?;
    let file_id = file.id.ok_or("no file id")?;
    let batch: CreateBatchResponse =
        CreateBatchRequest::new(file_id, BatchEndpoint::ChatCompletions)
            .with_auto_delete_input_file(true)
            .send_via(&client)
            .await?;
    println!("created batch: {:?}", batch.id);
    Ok(())
}
