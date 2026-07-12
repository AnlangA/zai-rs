//! Synthesize speech as a complete WAV response or a typed PCM stream.

use std::path::PathBuf;

use zai_rs::{
    client::ZaiClient,
    model::text_to_audio::{GlmTts, TextToAudioRequest, TtsAudioFormat, Voice},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output_arg = std::env::args().nth(1).map(PathBuf::from);
    let streaming = std::env::args().any(|argument| argument == "--stream");
    let output = output_arg.unwrap_or_else(|| {
        PathBuf::from(if streaming {
            "tts_output.pcm"
        } else {
            "tts_output.wav"
        })
    });

    let client = ZaiClient::from_env()?;
    let request = TextToAudioRequest::new(GlmTts {})
        .with_input("你好，这是由 zai-rs 生成的一段语音。")
        .with_voice(Voice::Tongtong);
    let bytes = if streaming {
        let mut stream = request.enable_stream().stream_via(&client).await?;
        let mut audio = Vec::new();
        while let Some(chunk) = stream.next().await {
            audio.extend_from_slice(&chunk?);
        }
        audio
    } else {
        request
            .with_response_format(TtsAudioFormat::Wav)
            .send_via(&client)
            .await?
            .to_vec()
    };
    tokio::fs::write(&output, &bytes).await?;
    println!("saved {} bytes to {}", bytes.len(), output.display());

    Ok(())
}
