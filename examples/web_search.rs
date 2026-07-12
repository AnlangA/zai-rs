//! Search the web through the HTTP tools API.

use zai_rs::{
    client::ZaiClient,
    tool::web_search::{SearchEngine, WebSearchRequest},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let query = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "Rust programming language".to_owned());

    let client = ZaiClient::from_env()?;
    let request = WebSearchRequest::new(query, SearchEngine::SearchStd);
    let response = request.send_via(&client).await?;
    println!("{response:#?}");
    Ok(())
}
