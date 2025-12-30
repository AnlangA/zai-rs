use std::{fs::File, io::Write};

use base64::Engine;
use zai_rs::model::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let model = GLM4_voice {};
    let key =
        std::env::var("ZHIPU_API_KEY").expect("ZHIPU_API_KEY environment variable must be set");

    let text_contxt = VoiceRichContent::text("复述一遍");
    // Read the audio file
    let audio_data = std::fs::read("data/你好.wav")?;
    // Create audio content from the local WAV file
    let audio_content = VoiceRichContent::input_audio(audio_data, VoiceFormat::WAV);
    let voice_message = VoiceMessage::new_user()
        .add_user(text_contxt)
        .add_user(audio_content);

    let client = ChatCompletion::new(model, voice_message, key);

    match tokio::time::timeout(tokio::time::Duration::from_secs(30), client.send()).await {
        Ok(Ok(body)) => {
            // Success responses are JSON; parsed as struct

            let audio_b64 = body
                .choices()
                .and_then(|cs| cs.first())
                .and_then(|c| c.message().audio())
                .and_then(|a| a.data());

            if let Some(b64) = audio_b64 {
                let audio_bytes = base64::engine::general_purpose::STANDARD.decode(b64)?;
                let filename = format!("data/response_{}.wav", chrono::Utc::now().timestamp());
                File::create(&filename)?.write_all(&audio_bytes)?;
                println!("Audio saved to: {}", filename);
            }
        },
        Ok(Err(e)) => {
            return Err(e.into());
        },
        Err(_) => {
            println!("Request timed out after 30 seconds");
            return Err("Request timed out".into());
        },
    }
    Ok(())
}
