use zai_rs::model::voice_list::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    // Build request: optionally filter by name/type
    let query = VoiceListQuery::new()
        // .with_voice_name("my_custom")
        // .with_voice_type(VoiceType::Private)
        ;

    let client = VoiceListRequest::new(key).with_query(query);

    let body: VoiceListResponse = client.send().await?;
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
