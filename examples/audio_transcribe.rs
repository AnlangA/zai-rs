use zai_rs::model::audio_to_text::audio_asr_model::GlmAsr;
use zai_rs::model::audio_to_text::response::AudioTranscriptionResponse;
use zai_rs::model::audio_to_text::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Set your API key in env: ZHIPU_API_KEY
    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    // Use local wav file as input
    let file_path = "data/你好.wav";

    // Build and send request
    let model = GlmAsr {};
    let client = AudioTranscriptionRequest::new(model, key)
        .with_file_path(file_path)
        .with_temperature(0.95)
        .with_stream(false);

    let body: AudioTranscriptionResponse = client.send().await?;
    println!("{:#?}", body);

    Ok(())
}
