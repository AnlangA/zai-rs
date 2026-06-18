use zai_rs::model::audio_to_text::{model::GlmAsr, response::AudioToTextResponse, *};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("RUST_LOG").is_some() {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .try_init();
    }

    // Set your API key in env: ZHIPU_API_KEY
    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    // Use local wav file as input
    let file_path = "data/你好.wav";

    // Build and send request
    let model = GlmAsr {};
    let client = AudioToTextRequest::new(model, key)
        .with_file_path(file_path)
        .with_temperature(0.95)
        .with_stream(false);

    let body: AudioToTextResponse = client.send().await?;
    println!("{:#?}", body);

    Ok(())
}
