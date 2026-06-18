use zai_rs::model::ocr::{
    request::{OcrLanguageType, OcrToolType},
    response::OcrResponse,
    *,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("RUST_LOG").is_some() {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .try_init();
    }

    // Set your API key in env: ZHIPU_API_KEY
    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    // Use local image file as input
    let file_path = "data/ocr_example.png";

    tracing::trace!("=== OCR Handwriting Recognition Example ===\n");

    // Build and send OCR request
    let client = OcrRequest::new(key)
        .with_file_path(file_path)
        .with_tool_type(OcrToolType::HandWrite)
        .with_language_type(OcrLanguageType::ChnEng)
        .with_probability(true);

    tracing::trace!("Sending OCR request for image: {}\n", file_path);

    let response: OcrResponse = client.send().await?;

    tracing::trace!("OCR Recognition Result:\n");
    tracing::trace!("Task ID: {:?}", response.task_id);
    tracing::trace!("Status: {:?}", response.status);
    tracing::trace!("Message: {:?}", response.message);
    tracing::trace!("Number of Results: {:?}\n", response.words_result_num);

    if let Some(results) = response.words_result {
        for (idx, item) in results.iter().enumerate() {
            tracing::trace!("--- Text Block {} ---", idx + 1);

            if let Some(words) = &item.words {
                tracing::trace!("Text: {}", words);
            }

            if let Some(location) = &item.location {
                tracing::trace!(
                    "Location: left={}, top={}, width={}, height={}",
                    location.left.unwrap_or(0),
                    location.top.unwrap_or(0),
                    location.width.unwrap_or(0),
                    location.height.unwrap_or(0)
                );
            }

            if let Some(prob) = &item.probability {
                tracing::trace!(
                    "Confidence: avg={:.2}, var={:.2}, min={:.2}",
                    prob.average.unwrap_or(0.0),
                    prob.variance.unwrap_or(0.0),
                    prob.min.unwrap_or(0.0)
                );
            }

            tracing::trace!("");
        }
    }

    tracing::trace!("=== OCR Example Completed ===");

    Ok(())
}
