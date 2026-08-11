//! Stream a `glm-realtime-flash` response to a raw PCM file.
//!
//! The output is 24 kHz, mono, signed 16-bit little-endian PCM.

use std::{env, io::Write as _, path::PathBuf, time::Duration};

use futures_util::{FutureExt as _, StreamExt as _};
use tokio::io::AsyncWriteExt as _;
use zai_rs::{
    model::GLM_realtime_flash,
    realtime::{RealtimeClient, RealtimeModality, RealtimeTransportConfig, ServerEvent},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = env::var("ZHIPU_API_KEY")?;
    let mut args = env::args().skip(1);
    let prompt = args
        .next()
        .unwrap_or_else(|| "用一句话介绍你自己。".to_owned());
    let output_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("realtime_response.pcm"));

    let transport_config = RealtimeTransportConfig::builder()
        // Built-in connection attempts and retry waits share this total budget.
        .connect_timeout(Duration::from_secs(8))
        .inbound_idle_timeout(Duration::from_secs(120))
        .try_build()?;
    let session = RealtimeClient::new(key)
        .with_transport_config(transport_config)
        .session(GLM_realtime_flash {})
        .instructions("你是一个简洁、礼貌的中文语音助手。")
        .modalities([RealtimeModality::Audio])
        .build()
        .await?;

    // Subscribe before sending so no early transcript or audio chunk is lost.
    let result = {
        let mut events = session.events();
        let mut audio = session.audio_stream();

        tokio::time::timeout(Duration::from_secs(120), async {
            let mut output = tokio::fs::File::create(&output_path).await?;
            session.send_text(prompt).await?;
            session.create_response().await?;

            loop {
                tokio::select! {
                    event = events.next() => match event {
                        Some(Ok(ServerEvent::ResponseAudioTranscriptDelta { delta, .. })) => {
                            print!("{delta}");
                            std::io::stdout().flush()?;
                        },
                        Some(Ok(ServerEvent::ResponseDone { response })) => {
                            if response.status == "completed" {
                                break;
                            }
                            return Err(format!(
                                "realtime response ended with status {}",
                                response.status
                            ).into());
                        },
                        Some(Ok(ServerEvent::Error { error })) => {
                            return Err::<(), Box<dyn std::error::Error>>(
                                error.message.into()
                            );
                        },
                        Some(Ok(_)) => {},
                        Some(Err(error)) => return Err(error.into()),
                        None => return Err("realtime event stream ended unexpectedly".into()),
                    },
                    chunk = audio.next() => match chunk {
                        Some(Ok(chunk)) => output.write_all(&chunk.data).await?,
                        Some(Err(error)) => return Err(error.into()),
                        None => return Err("realtime audio stream ended unexpectedly".into()),
                    },
                }
            }

            // Audio events are queued before the matching done event; drain any
            // ready chunks that lost the final `select!` race.
            while let Some(Some(chunk)) = audio.next().now_or_never() {
                output.write_all(&chunk?.data).await?;
            }
            output.flush().await?;
            println!("\nsaved to {}", output_path.display());
            Ok::<(), Box<dyn std::error::Error>>(())
        })
        .await
    };

    session.close().await?;
    result??;
    Ok(())
}
