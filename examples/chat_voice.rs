use zai_rs::model::chat_base_response::ChatCompletionResponse;
use zai_rs::model::*;

use base64::Engine;
use chrono;
use std::fs::File;
use std::io::Write;
use tokio;
use zai_rs::client::http::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let model = GLM4_voice {};
    let key = std::env::var("ZHIPU_API_KEY").unwrap();

    let text_contxt = VoiceRichContent::text("复述一遍");
    // Read the audio file
    let audio_data = std::fs::read("data/你好.wav").unwrap();
    // Create audio content from the local WAV file
    let audio_content = VoiceRichContent::input_audio(audio_data, VoiceFormat::WAV);
    let voice_message = VoiceMessage::new_user()
        .add_user(text_contxt)
        .add_user(audio_content);

    let client = ChatCompletion::new(model, voice_message, key);

    println!("Sending request to GLM-4-Voice API...");
    match tokio::time::timeout(tokio::time::Duration::from_secs(30), client.post()).await {
        Ok(Ok(resp)) => {
            println!("Response status: {}", resp.status());
            println!("Response headers: {:?}", resp.headers());

            let status = resp.status();
            let text = resp
                .text()
                .await
                .unwrap_or_else(|_| "Failed to get response text".to_string());

            if status.is_success() {
                // Try to parse with the struct first
                match serde_json::from_str::<ChatCompletionResponse>(&text) {
                    Ok(body) => {
                        println!("Successfully parsed response with struct");

                        // Extract and save audio data
                        if let Some(choices) = body.choices() {
                            for choice in choices {
                                if let Some(audio_content) = choice.message().audio() {
                                    if let Some(audio_id) = audio_content.id() {
                                        println!("Audio ID: {}", audio_id);
                                    }

                                    if let Some(base64_data) = audio_content.data() {
                                        println!(
                                            "Found audio data, length: {} bytes",
                                            base64_data.len()
                                        );

                                        // Decode base64 audio data
                                        match base64::engine::general_purpose::STANDARD
                                            .decode(base64_data)
                                        {
                                            Ok(audio_bytes) => {
                                                println!(
                                                    "Successfully decoded audio data: {} bytes",
                                                    audio_bytes.len()
                                                );

                                                // Save to file
                                                let filename = format!(
                                                    "data/response_{}.wav",
                                                    chrono::Utc::now().timestamp()
                                                );
                                                match File::create(&filename) {
                                                    Ok(mut file) => {
                                                        match file.write_all(&audio_bytes) {
                                                            Ok(_) => {
                                                                println!(
                                                                    "Audio saved to: {}",
                                                                    filename
                                                                );
                                                            }
                                                            Err(e) => {
                                                                println!("Failed to write audio file: {}", e);
                                                            }
                                                        }
                                                    }
                                                    Err(e) => {
                                                        println!(
                                                            "Failed to create audio file: {}",
                                                            e
                                                        );
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                println!(
                                                    "Failed to decode base64 audio data: {}",
                                                    e
                                                );
                                            }
                                        }
                                    } else {
                                        println!("No audio data found in response");
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("Failed to parse response JSON with struct: {}", e);

                        // Fallback: parse as raw JSON to extract audio data
                        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&text) {
                            println!("Successfully parsed response as raw JSON");

                            if let Some(choices) =
                                json_value.get("choices").and_then(|c| c.as_array())
                            {
                                for choice in choices {
                                    if let Some(message) =
                                        choice.get("message").and_then(|m| m.as_object())
                                    {
                                        if let Some(audio) =
                                            message.get("audio").and_then(|a| a.as_object())
                                        {
                                            if let Some(audio_id) =
                                                audio.get("id").and_then(|i| i.as_str())
                                            {
                                                println!("Audio ID: {}", audio_id);
                                            }

                                            if let Some(base64_data) =
                                                audio.get("data").and_then(|d| d.as_str())
                                            {
                                                println!(
                                                    "Found audio data, length: {} bytes",
                                                    base64_data.len()
                                                );

                                                // Decode base64 audio data
                                                match base64::engine::general_purpose::STANDARD
                                                    .decode(base64_data)
                                                {
                                                    Ok(audio_bytes) => {
                                                        println!("Successfully decoded audio data: {} bytes", audio_bytes.len());

                                                        // Save to file
                                                        let filename = format!(
                                                            "data/response_{}.wav",
                                                            chrono::Utc::now().timestamp()
                                                        );
                                                        match File::create(&filename) {
                                                            Ok(mut file) => {
                                                                match file.write_all(&audio_bytes) {
                                                                    Ok(_) => {
                                                                        println!(
                                                                            "Audio saved to: {}",
                                                                            filename
                                                                        );
                                                                    }
                                                                    Err(e) => {
                                                                        println!("Failed to write audio file: {}", e);
                                                                    }
                                                                }
                                                            }
                                                            Err(e) => {
                                                                println!("Failed to create audio file: {}", e);
                                                            }
                                                        }
                                                    }
                                                    Err(e) => {
                                                        println!("Failed to decode base64 audio data: {}", e);
                                                    }
                                                }
                                            } else {
                                                println!("No audio data found in response");
                                            }
                                        }
                                    }
                                }
                            } else {
                                println!("No choices found in response");
                            }
                        } else {
                            println!("Failed to parse response as raw JSON");
                            println!("Response text: {}", text);
                        }
                    }
                }
            } else {
                println!("Request failed with status: {}", status);
                println!("Response text: {}", text);
            }
        }
        Ok(Err(e)) => {
            println!("Request failed: {}", e);
            return Err(e.into());
        }
        Err(_) => {
            println!("Request timed out after 30 seconds");
            return Err("Request timed out".into());
        }
    }
    Ok(())
}
