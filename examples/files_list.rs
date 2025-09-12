use zai_rs::file::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    // Build query (all optional)
    let query = FileListQuery::new()
        .with_purpose(FilePurpose::FileExtract);

    let list = FileListRequest::new(key.clone()).with_query(query);
    let body: FileListResponse = list.send().await?;

    println!("object: {:?}", body.object);
    println!("has_more: {:?}", body.has_more);
    if let Some(data) = &body.data {
        println!("files: {}", data.len());
        for (i, f) in data.iter().enumerate() {
            println!(
                "#{}: id={:?} filename={:?} bytes={:?} purpose={:?}",
                i + 1,
                f.id,
                f.filename,
                f.bytes,
                f.purpose
            );
        }
    }

    Ok(())
}
