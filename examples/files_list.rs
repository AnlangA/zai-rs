use zai_rs::file::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("RUST_LOG").is_some() {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .try_init();
    }

    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    // Build query (all optional)
    let query = FileListQuery::new().with_purpose(FilePurpose::FileExtract);

    let list = FileListRequest::new(key.clone()).with_query(query);
    let body: FileListResponse = list.send().await?;

    tracing::trace!("object: {:?}", body.object);
    tracing::trace!("has_more: {:?}", body.has_more);
    if let Some(data) = &body.data {
        tracing::trace!("files: {}", data.len());
        for (i, f) in data.iter().enumerate() {
            tracing::trace!(
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
