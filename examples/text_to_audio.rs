use zai_rs::{
    client::http::*,
    model::text_to_audio::{model::GlmTts, *},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    // Build TTS request
    let model = GlmTts {};
    let input =
        "你好，我是你的朋友，我会rap:\"床前明月光，嘿嘿！疑是地上霜。举头望明月，低头思故乡。\"";
    let client = TextToAudioRequest::new(model, key)
        .with_input(input)
        .with_voice(Voice::Tongtong)
        .with_speed(1.0)
        .with_volume(1.0)
        .with_response_format(TtsAudioFormat::Wav)
        .with_watermark_enabled(false);

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
