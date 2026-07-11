use zai_rs::client::ZaiClient;
use zai_rs::model::voice_delete::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Credentials and transport come from the environment via ZaiClient (P05).
    let client = ZaiClient::from_env()?;

    // Example voice id to delete
    let voice = "voice_clone_20240315_143052_001";

    let request = VoiceDeleteRequest::new(voice).with_request_id("voice_delete_req_001");

    let body: VoiceDeleteResponse = request.send_via(&client).await?;
    println!("{body:#?}");

    Ok(())
}
