use zai_rs::client::v2::ZaiClient;
use zai_rs::model::text_rerank::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let request = RerankRequest::new(
        "什么是Rust",
        vec![
            "Rust是一门系统编程语言".to_string(),
            "Python是解释型语言".to_string(),
        ],
    );
    let resp = request.send_via(&client).await?;
    println!("{resp:#?}");
    Ok(())
}
