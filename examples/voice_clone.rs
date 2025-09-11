use zai_rs::model::voice_clone::*;
use zai_rs::model::voice_clone::model::CogTtsClone;
use zai_rs::model::voice_clone::response::VoiceCloneResponse;
use zai_rs::client::http::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    // Example values from the spec
    let model = CogTtsClone {};
    let voice_name = "my_custom_voice_001";
    let text = "你好，这是一段示例音频的文本内容，用于音色复刻参考。";
    let input = "欢迎使用我们的音色复刻服务，这将生成与示例音频相同音色的语音。";
    
    //通过文件上传接口上传音频文件，获取file_id。暂时用zhipu官方提供的。
    let file_id = "abcf4765-0d08-5cbd-8bd8-6867f76166cc";

    let client = VoiceCloneRequest::new(model, key, voice_name, input, file_id)
        .with_request_id("voice_clone_req_001")
        .with_text(text);

    let resp = client.post().await?;
    let status = resp.status();
    if !status.is_success() {
        let txt = resp.text().await.unwrap_or_default();
        eprintln!("Request failed: {}\n{}", status, txt);
        return Ok(());
    }

    let body: VoiceCloneResponse = resp.json().await?;
    println!("{:#?}", body);

    Ok(())
}

