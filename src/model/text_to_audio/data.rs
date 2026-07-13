use std::{
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use base64::Engine as _;
use futures_util::{Stream, StreamExt};
use validator::Validate;

use super::{
    super::traits::{StreamOff, StreamOn, StreamState, TextToAudio},
    request::{TextToAudioBody, TtsAudioFormat, TtsEncodeFormat, Voice},
};
use crate::{
    ZaiError, ZaiResult,
    client::{ZaiClient, error::codes},
};

/// Typed PCM byte stream returned by [`TextToAudioRequest::stream_via`].
pub struct TextToAudioStream {
    inner: crate::model::sse_parser::DecodedSseStream<bytes::Bytes>,
}

impl TextToAudioStream {
    /// Await the next decoded PCM chunk, or `None` after `[DONE]`.
    ///
    /// Protocol and decode failures are yielded once before termination.
    pub async fn next(&mut self) -> Option<ZaiResult<bytes::Bytes>> {
        self.inner.next().await
    }
}

impl Stream for TextToAudioStream {
    type Item = ZaiResult<bytes::Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(context)
    }
}

/// Type-state request builder for text-to-speech synthesis.
///
/// Non-streaming requests return complete WAV or PCM bytes through
/// [`send_via`](Self::send_via). [`enable_stream`](Self::enable_stream)
/// switches to PCM-only SSE output decoded by [`stream_via`](Self::stream_via).
pub struct TextToAudioRequest<N, S = StreamOff>
where
    N: TextToAudio,
    S: StreamState,
{
    body: TextToAudioBody<N>,
    _stream: PhantomData<S>,
}

impl<N> TextToAudioRequest<N, StreamOff>
where
    N: TextToAudio,
{
    /// Create a non-streaming TTS request for the selected model.
    pub fn new(model: N) -> Self {
        Self {
            body: TextToAudioBody::new(model),
            _stream: PhantomData,
        }
    }

    /// Select WAV or PCM for a complete non-streaming response.
    pub fn with_response_format(mut self, format: TtsAudioFormat) -> Self {
        self.body = self.body.with_response_format(format);
        self
    }

    /// Switch to PCM-only SSE output with base64 payloads by default.
    pub fn enable_stream(self) -> TextToAudioRequest<N, StreamOn> {
        TextToAudioRequest {
            body: self.body.with_stream(true),
            _stream: PhantomData,
        }
    }

    /// Submit a non-streaming request and return its complete audio bytes.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<bytes::Bytes> {
        self.validate()?;
        let route = crate::client::routes::AUDIO_SYNTHESIZE;
        client.operation(route).send_json_bytes(&self.body).await
    }
}

impl<N> TextToAudioRequest<N, StreamOn>
where
    N: TextToAudio,
{
    /// Select standard base64 or hexadecimal SSE audio payloads.
    pub fn with_encode_format(mut self, format: TtsEncodeFormat) -> Self {
        self.body = self.body.with_encode_format(format);
        self
    }

    /// Return to non-streaming PCM output and remove `encode_format`.
    pub fn disable_stream(self) -> TextToAudioRequest<N, StreamOff> {
        TextToAudioRequest {
            body: self.body.with_stream(false),
            _stream: PhantomData,
        }
    }

    /// Submit the request and yield decoded PCM chunks from its SSE response.
    ///
    /// The streaming POST is never retried or redirected. A missing `[DONE]`,
    /// malformed encoded chunk, in-band business error, oversized event, or
    /// idle timeout is returned once and then terminates the stream.
    pub async fn stream_via(&self, client: &ZaiClient) -> ZaiResult<TextToAudioStream> {
        self.validate()?;
        let Some(encoding) = self.body.encode_format else {
            return Err(ZaiError::ApiError {
                code: codes::SDK_VALIDATION,
                message: "streaming TTS requires encode_format".to_string(),
            });
        };
        let route = crate::client::routes::AUDIO_SYNTHESIZE;
        let raw = client.operation(route).send_sse_json(&self.body).await?;
        let inner = crate::model::sse_parser::decode_required_done_stream(raw, move |payload| {
            decode_audio_payload(payload, encoding)
        });
        Ok(TextToAudioStream { inner })
    }
}

impl<N, S> TextToAudioRequest<N, S>
where
    N: TextToAudio,
    S: StreamState,
{
    /// Borrow the request body without exposing its type-state invariants for
    /// mutation.
    pub fn body(&self) -> &TextToAudioBody<N> {
        &self.body
    }

    /// Set the required input text (`1..=1024` Unicode scalar values).
    pub fn with_input(mut self, input: impl Into<String>) -> Self {
        self.body = self.body.with_input(input);
        self
    }

    /// Select a built-in or cloned voice.
    pub fn with_voice(mut self, voice: Voice) -> Self {
        self.body = self.body.with_voice(voice);
        self
    }

    /// Set playback speed in `0.5..=2.0`.
    pub fn with_speed(mut self, speed: f32) -> Self {
        self.body = self.body.with_speed(speed);
        self
    }

    /// Set playback volume in `0 < volume <= 10`.
    pub fn with_volume(mut self, volume: f32) -> Self {
        self.body = self.body.with_volume(volume);
        self
    }

    /// Enable or disable the service's audio watermark.
    pub fn with_watermark_enabled(mut self, enabled: bool) -> Self {
        self.body = self.body.with_watermark_enabled(enabled);
        self
    }

    /// Validate all wire constraints without network I/O.
    pub fn validate(&self) -> ZaiResult<()> {
        self.body.validate().map_err(ZaiError::from)
    }
}

fn decode_audio_payload(payload: &[u8], encoding: TtsEncodeFormat) -> ZaiResult<bytes::Bytes> {
    // The frozen TTS contract defines each SSE `data:` payload as the direct
    // `encode_format` text, not as a JSON envelope. Decode that exact payload;
    // JSON strings or surrounding whitespace are intentionally rejected.
    let encoded = std::str::from_utf8(payload)
        .map_err(|_| stream_protocol_error("TTS SSE audio payload must be UTF-8 encoded text"))?;
    if encoded.is_empty() {
        return Err(stream_protocol_error(
            "TTS SSE audio payload must not be empty",
        ));
    }
    let decoded = match encoding {
        TtsEncodeFormat::Base64 => base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|_| stream_protocol_error("invalid base64 TTS SSE audio payload"))?,
        TtsEncodeFormat::Hex => decode_hex(encoded)?,
    };
    Ok(bytes::Bytes::from(decoded))
}

fn decode_hex(encoded: &str) -> ZaiResult<Vec<u8>> {
    if !encoded.len().is_multiple_of(2) {
        return Err(stream_protocol_error(
            "hex TTS SSE audio payload must have an even length",
        ));
    }
    encoded
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = hex_nibble(pair[0])?;
            let low = hex_nibble(pair[1])?;
            Ok((high << 4) | low)
        })
        .collect()
}

fn hex_nibble(byte: u8) -> ZaiResult<u8> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(stream_protocol_error(
            "invalid hexadecimal TTS SSE audio payload",
        )),
    }
}

fn stream_protocol_error(message: impl Into<String>) -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_IO,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::text_to_audio::GlmTts;

    #[test]
    fn streaming_type_state_forces_pcm_and_scopes_encoding() {
        let request = TextToAudioRequest::new(GlmTts {})
            .with_input("hello")
            .with_response_format(TtsAudioFormat::Wav)
            .enable_stream();
        assert!(request.body().is_streaming());
        assert_eq!(request.body().response_format(), TtsAudioFormat::Pcm);
        assert_eq!(
            request.body().encode_format(),
            Some(TtsEncodeFormat::Base64)
        );

        let request = request
            .with_encode_format(TtsEncodeFormat::Hex)
            .disable_stream();
        assert!(!request.body().is_streaming());
        assert_eq!(request.body().response_format(), TtsAudioFormat::Pcm);
        assert_eq!(request.body().encode_format(), None);
    }

    #[test]
    fn base64_and_hex_payloads_decode_to_bytes() {
        assert_eq!(
            decode_audio_payload(b"AAEC/w==", TtsEncodeFormat::Base64).unwrap(),
            bytes::Bytes::from_static(&[0, 1, 2, 255])
        );
        assert_eq!(
            decode_audio_payload(b"000102fF", TtsEncodeFormat::Hex).unwrap(),
            bytes::Bytes::from_static(&[0, 1, 2, 255])
        );
        assert!(decode_audio_payload(b"%%%", TtsEncodeFormat::Base64).is_err());
        assert!(decode_audio_payload(br#""AAEC/w==""#, TtsEncodeFormat::Base64).is_err());
        assert!(decode_audio_payload(b" AAEC/w==", TtsEncodeFormat::Base64).is_err());
        assert!(decode_audio_payload(b"123", TtsEncodeFormat::Hex).is_err());
        assert!(decode_audio_payload(b"12xz", TtsEncodeFormat::Hex).is_err());
    }
}
