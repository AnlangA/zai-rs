//! Invoke an LLM application with one text input and print the response.

use zai_rs::{
    ZaiResult,
    client::ZaiClient,
    services::applications::{
        ApplicationInvokeContent, ApplicationInvokeMessage, ApplicationInvokeRequest,
        ApplicationInvokeResponse,
    },
};

#[tokio::main]
async fn main() -> ZaiResult<()> {
    let app_id = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: application <app_id> [prompt]");
        std::process::exit(2);
    });
    let prompt = std::env::args().nth(2).unwrap_or_else(|| "你好".to_owned());

    let client = ZaiClient::from_env()?;
    let response: ApplicationInvokeResponse = ApplicationInvokeRequest::new(
        app_id,
        vec![ApplicationInvokeMessage::new(vec![
            ApplicationInvokeContent::new("input", prompt),
        ])],
    )
    .send_via(&client)
    .await?;
    println!("{response:#?}");

    Ok(())
}
