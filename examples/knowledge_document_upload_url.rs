use zai_rs::knowledge::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = env_logger::try_init();
    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    // Args: <knowledge_id> <url1> [url2] [url3] ...
    let knowledge_id = std::env::args()
        .nth(1)
        .expect("Usage: knowledge_document_upload_url <knowledge_id> <url1> [url2] ...");
    let urls: Vec<String> = std::env::args().skip(2).collect();
    if urls.is_empty() {
        panic!("Please provide at least one url");
    }

    let mut body = UploadUrlBody::new(knowledge_id);
    for u in urls {
        body = body.add_url(u);
    }

    let req = DocumentUploadUrlRequest::new(key, body);
    let resp: UploadUrlResponse = req.send().await?;

    println!(
        "code={:?} message={:?} timestamp={:?}",
        resp.code, resp.message, resp.timestamp
    );
    if let Some(data) = &resp.data {
        if let Some(ok) = &data.success_infos {
            for s in ok.iter() {
                println!("success: doc_id={:?} url={:?}", s.document_id, s.url);
            }
        }
        if let Some(fails) = &data.failed_infos {
            for f in fails.iter() {
                println!("failed: url={:?} reason={:?}", f.url, f.fail_reason);
            }
        }
    }

    Ok(())
}
