use zai_rs::client::ZaiClient;
use zai_rs::knowledge::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;

    // Args: <knowledge_id> <file1> [file2] [file3] ...
    let knowledge_id = std::env::args()
        .nth(1)
        .expect("Usage: knowledge_document_upload_file <knowledge_id> <file1> [file2] ...");
    let files: Vec<String> = std::env::args().skip(2).collect();
    if files.is_empty() {
        panic!("Please provide at least one file path");
    }

    let mut req = DocumentUploadFileRequest::new(knowledge_id);
    for f in files {
        req = req.add_file_path(f);
    }

    // Minimal example: dynamic parse, no extra options
    let resp: UploadFileResponse = req.send_via(&client).await?;

    println!(
        "code={:?} message={:?} timestamp={:?}",
        resp.code, resp.message, resp.timestamp
    );
    if let Some(data) = &resp.data {
        if let Some(ok) = &data.success_infos {
            for s in ok.iter() {
                println!("success: doc_id={:?} file={:?}", s.document_id, s.file_name);
            }
        }
        if let Some(fails) = &data.failed_infos {
            for f in fails.iter() {
                println!("failed: file={:?} reason={:?}", f.file_name, f.fail_reason);
            }
        }
    }

    Ok(())
}
