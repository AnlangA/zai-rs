//! Transcribe a local WAV or MP3 file, optionally through typed SSE events.
//!
//! Defaults to `data/你好.wav`; pass a path to use another file.

use zai_rs::{
    client::ZaiClient,
    model::audio_to_text::{AudioToTextRequest, AudioToTextResponse, GlmAsr},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file_path = std::env::args()
        .skip(1)
        .find(|argument| argument != "--stream")
        .unwrap_or_else(|| "data/你好.wav".to_owned());
    let streaming = std::env::args().any(|argument| argument == "--stream");

    let client = ZaiClient::from_env()?;
    let request = AudioToTextRequest::new(GlmAsr {}).with_file_path(&file_path);
    if streaming {
        let mut stream = request.enable_stream().stream_via(&client).await?;
        while let Some(event) = stream.next().await {
            println!("{:#?}", event?);
        }
    } else {
        let body: AudioToTextResponse = request.send_via(&client).await?;
        println!("{body:#?}");
    }

    Ok(())
}
