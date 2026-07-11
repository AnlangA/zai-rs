use zai_rs::client::v2::ZaiClient;
use zai_rs::tool::web_search::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let request = WebSearchRequest::new(
        "Rust programming language".to_string(),
        SearchEngine::SearchStd,
    );
    match request.send_via(&client).await {
        Ok(resp) => println!("{resp:#?}"),
        Err(e) => eprintln!("Error: {e}"),
    }
    Ok(())
}
