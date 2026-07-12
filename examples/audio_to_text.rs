use zai_rs::client::ZaiClient;
use zai_rs::model::audio_to_text::{AudioToTextResponse, GlmAsr, *};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Credentials and transport come from the environment via ZaiClient (P05).
    let client = ZaiClient::from_env()?;

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
    let request = AudioToTextRequest::new(model)
        .with_file_path(&file_path)
        .with_stream(false);

    let body: AudioToTextResponse = request.send_via(&client).await?;
    println!("{body:#?}");

    Ok(())
}
