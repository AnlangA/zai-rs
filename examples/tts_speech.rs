use zai_rs::client::http::*;
use zai_rs::model::audio_to_speech::tts_model::CogTts;
use zai_rs::model::audio_to_speech::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    // Build TTS request
    let model = CogTts {};
    let input = "你好，今天天气怎么样";
    let client = TtsSpeechRequest::new(model, key)
        .with_input(input)
        .with_voice(TtsVoice::Tongtong)
        .with_speed(1.0)
        .with_volume(1.0)
        .with_response_format(TtsAudioFormat::Wav)
        .with_watermark_enabled(true);

    // Send and write audio to file
    let resp = client.post().await?;
    let status = resp.status();
    if !status.is_success() {
        let txt = resp.text().await.unwrap_or_default();
        eprintln!("Request failed: {}\n{}", status, txt);
        return Ok(());
    }

    let bytes = resp.bytes().await?;
    std::fs::create_dir_all("out").ok();
    std::fs::write("out/tts_output.wav", &bytes)?;
    println!("Saved to out/tts_output.wav ({} bytes)", bytes.len());

    Ok(())
}
