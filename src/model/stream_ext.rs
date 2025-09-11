use crate::client::http::HttpClient;
use crate::model::chat_stream_response::ChatStreamResponse;
use crate::model::traits::SseStreamable;
use futures::{stream, Stream, StreamExt};
use log::info;
use std::pin::Pin;

// A shared, typed streaming extension for "chat-like" endpoints that stream ChatStreamResponse
pub trait StreamChatLikeExt: SseStreamable + HttpClient {
    // 1) Async-closure callback API (simple and friendly)
    fn stream_for_each<'a, F, Fut>(
        &'a mut self,
        mut on_chunk: F,
    ) -> impl core::future::Future<Output = anyhow::Result<()>> + 'a
    where
        F: FnMut(ChatStreamResponse) -> Fut + 'a,
        Fut: core::future::Future<Output = anyhow::Result<()>> + 'a,
    {
        async move {
            let resp = self.post().await?;
            let mut stream = resp.bytes_stream();
            let mut buf: Vec<u8> = Vec::new();

            while let Some(next) = stream.next().await {
                let bytes = match next {
                    Ok(b) => b,
                    Err(e) => return Err(anyhow::anyhow!("Stream error: {}", e)),
                };
                buf.extend_from_slice(&bytes);
                while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                    let line_vec: Vec<u8> = buf.drain(..=pos).collect();
                    let mut line = &line_vec[..];
                    if line.ends_with(b"\n") {
                        line = &line[..line.len() - 1];
                    }
                    if line.ends_with(b"\r") {
                        line = &line[..line.len() - 1];
                    }
                    if line.is_empty() {
                        continue;
                    }
                    const PREFIX: &[u8] = b"data: ";
                    if line.starts_with(PREFIX) {
                        let rest = &line[PREFIX.len()..];
                        info!("SSE data: {}", String::from_utf8_lossy(rest));
                        if rest == b"[DONE]" {
                            return Ok(());
                        }
                        if let Ok(chunk) = serde_json::from_slice::<ChatStreamResponse>(rest) {
                            on_chunk(chunk).await?;
                        }
                    }
                }
            }
            Ok(())
        }
    }

    // 2) Stream API（可组合/测试/复用友好）
    fn to_stream<'a>(
        &'a mut self,
    ) -> impl core::future::Future<
        Output = anyhow::Result<
            Pin<Box<dyn Stream<Item = anyhow::Result<ChatStreamResponse>> + Send + 'static>>,
        >,
    > + 'a {
        async move {
            let resp = self.post().await?;
            let byte_stream = resp.bytes_stream();

            // State: (byte_stream, buffer)
            let s = byte_stream;

            let out = stream::unfold((s, Vec::<u8>::new()), |(mut s, mut buf)| async move {
                loop {
                    // Process all complete lines currently in buffer
                    while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                        let line_vec: Vec<u8> = buf.drain(..=pos).collect();
                        let mut line = &line_vec[..];
                        if line.ends_with(b"\n") {
                            line = &line[..line.len() - 1];
                        }
                        if line.ends_with(b"\r") {
                            line = &line[..line.len() - 1];
                        }
                        if line.is_empty() {
                            continue;
                        }
                        const PREFIX: &[u8] = b"data: ";
                        if line.starts_with(PREFIX) {
                            let rest = &line[PREFIX.len()..];
                            info!("SSE data: {}", String::from_utf8_lossy(rest));
                            if rest == b"[DONE]" {
                                return None; // end stream gracefully
                            }
                            match serde_json::from_slice::<ChatStreamResponse>(rest) {
                                Ok(item) => return Some((Ok(item), (s, buf))),
                                Err(_) => { /* skip invalid json line */ }
                            }
                        }
                    }
                    // Need more bytes
                    match s.next().await {
                        Some(Ok(bytes)) => buf.extend_from_slice(&bytes),
                        Some(Err(e)) => {
                            return Some((Err(anyhow::anyhow!("Stream error: {}", e)), (s, buf)))
                        }
                        None => return None,
                    }
                }
            })
            .boxed();

            Ok(out)
        }
    }
}
