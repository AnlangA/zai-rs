use zai_rs::knowledge::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = env_logger::try_init();
    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    let resp: KnowledgeCapacityResponse = KnowledgeCapacityRequest::new(key).send().await?;

    println!(
        "code={:?} message={:?} timestamp={:?}",
        resp.code, resp.message, resp.timestamp
    );
    if let Some(data) = &resp.data {
        if let Some(used) = &data.used {
            println!("used: words={:?} bytes={:?}", used.word_num, used.length);
        }
        if let Some(total) = &data.total {
            println!("total: words={:?} bytes={:?}", total.word_num, total.length);
        }
    }
    Ok(())
}
