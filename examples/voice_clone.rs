//! Clone a voice from an audio file already uploaded to the Files API.

use zai_rs::{
    client::ZaiClient,
    model::voice_clone::{GlmTtsClone, VoiceCloneRequest, VoiceCloneResponse},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let file_id = args
        .next()
        .ok_or("usage: voice_clone <file-id> <voice-name> [reference-transcript]")?;
    let voice_name = args
        .next()
        .ok_or("usage: voice_clone <file-id> <voice-name> [reference-transcript]")?;

    let mut request = VoiceCloneRequest::new(
        GlmTtsClone {},
        voice_name,
        "你好，这是使用复刻音色生成的试听语音。",
        file_id,
    );
    if let Some(transcript) = args.next() {
        request = request.with_text(transcript);
    }

    let client = ZaiClient::from_env()?;
    let response: VoiceCloneResponse = request.send_via(&client).await?;
    println!("{response:#?}");

    Ok(())
}
