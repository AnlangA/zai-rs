use zai_rs::file::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("RUST_LOG").is_some() {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .try_init();
    }

    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    // file id to delete, pass as arg or hardcode for testing
    let file_id = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "1757561531_ec561569199641b3a5c556503a72cb79".to_string());

    let body: FileDeleteResponse = FileDeleteRequest::new(key, file_id).send().await?;
    tracing::trace!("{:#?}", body);

    Ok(())
}
