use zai_rs::model::voice_delete::*;
use zai_rs::client::http::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    // Example voice id to delete
    let voice = "voice_clone_20240315_143052_001";

    let client = VoiceDeleteRequest::new(key, voice)
        .with_request_id("voice_delete_req_001");

    let resp = client.post().await?;
    let status = resp.status();
    if !status.is_success() {
        let txt = resp.text().await.unwrap_or_default();
        eprintln!("Request failed: {}\n{}", status, txt);
        return Ok(());
    }

    let body: VoiceDeleteResponse = resp.json().await?;
    println!("{:#?}", body);

    Ok(())
}

