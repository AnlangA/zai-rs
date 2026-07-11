use zai_rs::client::ZaiClient;
use zai_rs::model::voice_list::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Credentials and transport come from the environment via ZaiClient (P05).
    let client = ZaiClient::from_env()?;

    // Build request: optionally filter by name/type
    let query = VoiceListQuery::new()
        // .with_voice_name("my_custom")
        // .with_voice_type(VoiceType::Private)
        ;

    let request = VoiceListRequest::new().with_query(query);

    let body: VoiceListResponse = request.send_via(&client).await?;
    if let Some(list) = body.voice_list.as_ref() {
        println!("voices: {}", list.len());
        for (i, item) in list.iter().enumerate() {
            println!("#{}: {:?}", i + 1, item);
        }
    } else {
        println!("voices: 0");
    }

    Ok(())
}
