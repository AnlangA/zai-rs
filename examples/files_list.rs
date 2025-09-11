use zai_rs::file::*;
use zai_rs::client::http::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    // Build query (all optional to match typical cURL)
    // Many backends require purpose to filter results; set to FileExtract to match recent uploads
    let query = FileListQuery::new()
        .with_purpose(FilePurpose::FileExtract)
        // .with_order(FileOrder::CreatedAt)
        // .with_limit(20)
        ;

    let client = FileListRequest::new(key.clone()).with_query(query);

    let resp = client.get().await?;
    let status = resp.status();
    if !status.is_success() {
        let txt = resp.text().await.unwrap_or_default();
        eprintln!("Request failed: {}\n{}", status, txt);
        return Ok(());
    }

    let body: FileListResponse = resp.json().await?;
    println!("object: {:?}", body.object);
    println!("has_more: {:?}", body.has_more);
    if let Some(data) = &body.data {
        println!("files: {}", data.len());
        for (i, f) in data.iter().enumerate() {
            println!("#{}: id={:?} filename={:?} bytes={:?} purpose={:?}",
                i + 1, f.id, f.filename, f.bytes, f.purpose);
        }
    }

    Ok(())
}

