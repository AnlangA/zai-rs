//! Recognize Chinese and English handwriting in a local image.
//!
//! Defaults to `data/ocr_example.png`; pass a path to use another image.

use zai_rs::{
    client::ZaiClient,
    model::ocr::{OcrLanguageType, OcrRequest, OcrResponse, OcrToolType},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/ocr_example.png".to_owned());

    let client = ZaiClient::from_env()?;
    let request = OcrRequest::new()
        .with_file_path(image)
        .with_tool_type(OcrToolType::HandWrite)
        .with_language_type(OcrLanguageType::ChnEng)
        .with_probability(true);
    let response: OcrResponse = request.send_via(&client).await?;
    println!("{response:#?}");

    Ok(())
}
