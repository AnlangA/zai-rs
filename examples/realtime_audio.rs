//! Realtime audio example — a server-VAD voice conversation over GLM-Realtime.
//!
//! Requires `ZHIPU_API_KEY`. Connects to the realtime WebSocket, sends a text
//! prompt, prints the streamed transcript, and collects the response audio to
//! `realtime_out.bin`.
//!
//! ```sh
//! ZHIPU_API_KEY=xxxxx.yyyyy cargo run --example realtime_audio -- "讲个冷笑话"
//! ```

use std::{env, fs::File, io::Write};

use futures_util::StreamExt;
use zai_rs::{
    ZaiResult,
    model::GLM4_voice,
    realtime::{RealtimeClient, ServerEvent, TurnDetectionType},
};

#[tokio::main]
async fn main() -> ZaiResult<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();

    let key = env::var("ZHIPU_API_KEY").expect("ZHIPU_API_KEY must be set to run realtime_audio");
    let prompt = env::args()
        .nth(1)
        .unwrap_or_else(|| "用一句话介绍你自己".to_string());

    let session = RealtimeClient::new(key)
        .session(GLM4_voice {})
        .turn_detection(TurnDetectionType::ServerVad)
        .instructions("你是一个简洁、礼貌的中文语音助手。")
        .build()
        .await?;

    println!("[realtime] connected (model={})", session.model_name());

    // Ask the question as text and trigger inference.
    session.send_text(&prompt).await?;
    session.create_response().await?;

    // Drive the two streams inside a block so their borrows of `session` end
    // before `close(self)` consumes it.
    {
        let mut audio_out =
            File::create("realtime_out.bin").expect("failed to create realtime_out.bin");
        let mut events = session.events();
        let mut audio = session.audio_stream();

        loop {
            tokio::select! {
                ev = events.next() => match ev {
                    Some(ServerEvent::ResponseAudioTranscriptDelta { delta, .. }) => {
                        print!("{delta}");
                        let _ = std::io::stdout().flush();
                    },
                    Some(ServerEvent::ResponseDone { response }) => {
                        println!(
                            "\n[realtime] response done (status={:?})",
                            response.status
                        );
                        break;
                    },
                    Some(ServerEvent::Error { error }) => {
                        eprintln!(
                            "[realtime] server error: {}",
                            error.message.as_deref().unwrap_or("(no message)")
                        );
                        break;
                    },
                    _ => {},
                },
                chunk = audio.next() => {
                    if let Some(chunk) = chunk {
                        let _ = audio_out.write_all(&chunk);
                    }
                },
            }
        }
    }

    session.close().await
}
