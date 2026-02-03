use zai_rs::model::ocr::{
    request::{OcrLanguageType, OcrToolType},
    response::OcrResponse,
    *,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Set your API key in env: ZHIPU_API_KEY
    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    // Use local image file as input
    let file_path = "data/ocr_example.png";

    println!("=== OCR Handwriting Recognition Example ===\n");

    // Build and send OCR request
    let client = OcrRequest::new(key)
        .with_file_path(file_path)
        .with_tool_type(OcrToolType::HandWrite)
        .with_language_type(OcrLanguageType::ChnEng)
        .with_probability(true);

    println!("Sending OCR request for image: {}\n", file_path);

    let response: OcrResponse = client.send().await?;

    println!("OCR Recognition Result:\n");
    println!("Task ID: {:?}", response.task_id);
    println!("Status: {:?}", response.status);
    println!("Message: {:?}", response.message);
    println!("Number of Results: {:?}\n", response.words_result_num);

    if let Some(results) = response.words_result {
        for (idx, item) in results.iter().enumerate() {
            println!("--- Text Block {} ---", idx + 1);

            if let Some(words) = &item.words {
                println!("Text: {}", words);
            }

            if let Some(location) = &item.location {
                println!(
                    "Location: left={}, top={}, width={}, height={}",
                    location.left.unwrap_or(0),
                    location.top.unwrap_or(0),
                    location.width.unwrap_or(0),
                    location.height.unwrap_or(0)
                );
            }

            if let Some(prob) = &item.probability {
                println!(
                    "Confidence: avg={:.2}, var={:.2}, min={:.2}",
                    prob.average.unwrap_or(0.0),
                    prob.variance.unwrap_or(0.0),
                    prob.min.unwrap_or(0.0)
                );
            }

            println!();
        }
    }

    println!("=== OCR Example Completed ===");

    Ok(())
}
