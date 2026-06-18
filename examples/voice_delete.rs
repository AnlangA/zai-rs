use zai_rs::model::voice_delete::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("RUST_LOG").is_some() {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .try_init();
    }

    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    // Example voice id to delete
    let voice = "voice_clone_20240315_143052_001";

    let client = VoiceDeleteRequest::new(key, voice).with_request_id("voice_delete_req_001");

    let body: VoiceDeleteResponse = client.send().await?;
    tracing::trace!("{:#?}", body);

    Ok(())
}
