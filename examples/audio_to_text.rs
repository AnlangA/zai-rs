use zai_rs::model::audio_to_text::{model::GlmAsr, response::AudioToTextResponse, *};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set your API key in env: ZHIPU_API_KEY
    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    // Read the input WAV/MP3 path from the first CLI argument.
    let file_path = match std::env::args().nth(1) {
        Some(p) => p,
        None => {
            eprintln!("usage: audio_to_text <audio-file.wav|mp3>");
            std::process::exit(2);
        },
    };

    // Build and send request
    let model = GlmAsr {};
    let client = AudioToTextRequest::new(model, key)
        .with_file_path(&file_path)
        .with_stream(false);

    let body: AudioToTextResponse = client.send().await?;
    println!("{body:#?}");

    Ok(())
}
