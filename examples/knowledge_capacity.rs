use zai_rs::knowledge::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("RUST_LOG").is_some() {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .try_init();
    }
    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    let resp: KnowledgeCapacityResponse = KnowledgeCapacityRequest::new(key).send().await?;

    tracing::trace!(
        "code={:?} message={:?} timestamp={:?}",
        resp.code,
        resp.message,
        resp.timestamp
    );
    if let Some(data) = &resp.data {
        if let Some(used) = &data.used {
            tracing::trace!("used: words={:?} bytes={:?}", used.word_num, used.length);
        }
        if let Some(total) = &data.total {
            tracing::trace!("total: words={:?} bytes={:?}", total.word_num, total.length);
        }
    }
    Ok(())
}
