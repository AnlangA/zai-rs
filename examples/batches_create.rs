use std::fs::File;
use std::io::Write;

use serde_json::json;
use zai_rs::batches::*;
use zai_rs::file::*;

fn make_jsonl_line(custom_id: &str, user_content: &str) -> String {
    // Build a single request line for /v4/chat/completions
    let v = json!({
        "custom_id": custom_id,
        "method": "POST",
        "url": "/v4/chat/completions",
        "body": {
            "model": "glm-4",
            "messages": [
                {"role": "system", "content": "你是一个意图分类器."},
                {"role": "user", "content": user_content}
            ]
        }
    });
    v.to_string()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = env_logger::try_init();
    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    // Step 1: Prepare a .jsonl file (each line is a request JSON)
    std::fs::create_dir_all("data")?;
    let path = "data/batch_demo.jsonl";
    let mut f = File::create(path)?;
    let lines = vec![
        make_jsonl_line("request-1", "订单处理速度太慢，等了很久才发货。"),
        make_jsonl_line(
            "request-2",
            "商品有点小瑕疵，不过客服处理得很快，总体满意。",
        ),
        make_jsonl_line("request-3", "这款产品性价比很高，非常满意。"),
        make_jsonl_line("request-4", "说明书写得不清楚，看了半天也不知道怎么用。"),
    ];
    for line in lines {
        writeln!(f, "{}", line)?;
    }

    // Step 2: Upload the .jsonl file with purpose=batch
    let upload = FileUploadRequest::new(key.clone(), FilePurpose::Batch, path)
        .with_content_type("application/jsonl");
    let file: FileObject = upload.send().await?;
    let file_id = file.id.ok_or_else(|| Box::<dyn std::error::Error>::from("missing file id"))?;

    // Step 3: Create the batch task using the send() interface
    let create = CreateBatchRequest::new(key.clone(), file_id, BatchEndpoint::ChatCompletions)
        .with_auto_delete_input_file(true);
    let batch: CreateBatchResponse = create.send().await?;

    println!(
        "created batch: id={:?} status={:?} input_file_id={:?}",
        batch.id, batch.status, batch.input_file_id
    );

    // Next steps:
    // - Poll status via list/get APIs
    // - Download output_file_id and error_file_id when available

    Ok(())
}
