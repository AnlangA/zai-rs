use zai_rs::model::voice_delete::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    // Example voice id to delete
    let voice = "voice_clone_20240315_143052_001";

    let client = VoiceDeleteRequest::new(key, voice).with_request_id("voice_delete_req_001");

    let body: VoiceDeleteResponse = client.send().await?;
    println!("{:#?}", body);

    Ok(())
}
