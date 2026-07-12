use zai_rs::client::ZaiClient;
use zai_rs::model::ocr::{OcrLanguageType, OcrResponse, OcrToolType, *};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Shared client reads ZHIPU_API_KEY from the environment.
    let client = ZaiClient::from_env()?;

    // Read the input image path from the first CLI argument.
    let file_path = match std::env::args().nth(1) {
        Some(p) => p,
        None => {
            eprintln!("usage: ocr <image-file.png|jpg>");
            std::process::exit(2);
        },
    };

    println!("=== OCR Handwriting Recognition Example ===\n");

    // Build and send OCR request (P05: credentials live on the client).
    let request = OcrRequest::new()
        .with_file_path(&file_path)
        .with_tool_type(OcrToolType::HandWrite)
        .with_language_type(OcrLanguageType::ChnEng)
        .with_probability(true);

    println!("Sending OCR request for image: {file_path}\n");

    let response: OcrResponse = request.send_via(&client).await?;

    println!("OCR Recognition Result:\n");
    println!("Task ID: {:?}", response.task_id);
    println!("Status: {:?}", response.status);
    println!("Message: {:?}", response.message);
    println!("Number of Results: {:?}\n", response.words_result_num);

    if let Some(results) = response.words_result {
        for (idx, item) in results.iter().enumerate() {
            println!("--- Text Block {} ---", idx + 1);

            if let Some(words) = &item.words {
                println!("Text: {words}");
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
