//! List voices, optionally filtering by voice name.

use zai_rs::{
    client::ZaiClient,
    model::voice_list::{VoiceListQuery, VoiceListRequest, VoiceListResponse},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut request = VoiceListRequest::new();
    if let Some(name) = std::env::args().nth(1) {
        request = request.with_query(VoiceListQuery::new().with_voice_name(name));
    }

    let client = ZaiClient::from_env()?;
    let response: VoiceListResponse = request.send_via(&client).await?;
    println!("{response:#?}");

    Ok(())
}
