//! Send a WAV voice message and save the model's decoded audio response.
//!
//! Defaults to `data/你好.wav` as input; pass a path to use another file.

use std::path::PathBuf;

use base64::Engine;
use zai_rs::{
    client::ZaiClient,
    model::{GLM4_voice, VoiceFormat, VoiceMessage, VoiceRichContent, chat::ChatCompletion},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let audio_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data/你好.wav"));
    let output_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("voice_response.wav"));

    let client = ZaiClient::from_env()?;
    let audio_data = tokio::fs::read(audio_path).await?;
    let audio_content = VoiceRichContent::input_audio(audio_data, VoiceFormat::WAV);
    let voice_message = VoiceMessage::new_user()
        .add_content(VoiceRichContent::text("请复述这段语音。"))
        .add_content(audio_content);
    let body = ChatCompletion::new(GLM4_voice {}, voice_message)
        .with_watermark_enabled(true)
        .send_via(&client)
        .await?;

    let encoded = body
        .choices()
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.message())
        .and_then(|message| message.audio())
        .and_then(|audio| audio.data())
        .ok_or("voice response did not contain audio")?;
    let bytes = base64::engine::general_purpose::STANDARD.decode(encoded)?;
    tokio::fs::write(&output_path, bytes).await?;
    println!("saved to {}", output_path.display());
    Ok(())
}
