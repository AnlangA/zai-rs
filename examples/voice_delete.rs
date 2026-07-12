//! Delete a previously cloned voice by ID.

use zai_rs::{
    client::ZaiClient,
    model::voice_delete::{VoiceDeleteRequest, VoiceDeleteResponse},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let voice_id = std::env::args()
        .nth(1)
        .ok_or("usage: voice_delete <voice-id>")?;

    let client = ZaiClient::from_env()?;
    let response: VoiceDeleteResponse = VoiceDeleteRequest::new(voice_id).send_via(&client).await?;
    println!("{response:#?}");

    Ok(())
}
