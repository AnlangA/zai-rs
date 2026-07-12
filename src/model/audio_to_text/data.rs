use std::{
    marker::PhantomData,
    path::{Path, PathBuf},
    pin::Pin,
    task::{Context, Poll},
};

use base64::Engine as _;
use futures_util::{Stream, StreamExt};
use validator::Validate;

use super::{
    super::traits::{AudioToText, StreamOff, StreamOn, StreamState},
    request::AudioToTextBody,
    response::{AudioToTextResponse, SpeechToTextEvent},
};
use crate::{
    ZaiError, ZaiResult,
    client::{ZaiClient, error::codes},
};

const ASR_FILE_MAX_BYTES: u64 = 25 * 1024 * 1024;
const ASR_BASE64_MAX_BYTES: u64 = ASR_FILE_MAX_BYTES.div_ceil(3) * 4;
const ASR_BASE64_MULTIPART_LIMIT: u64 =
    ASR_BASE64_MAX_BYTES + crate::client::transport::limits::MULTIPART_FIELD_BYTES_MAX + 64;

enum AudioInputRef<'a> {
    File(&'a Path),
    Base64(&'a str),
}

/// Authenticated typed response stream returned by
/// [`AudioToTextRequest::stream_via`].
pub struct SpeechToTextStream {
    inner: crate::model::sse_parser::DecodedSseStream<SpeechToTextEvent>,
}

impl SpeechToTextStream {
    /// Await the next transcription event, or `None` after `[DONE]`.
    ///
    /// Protocol and decode failures are yielded once before termination.
    pub async fn next(&mut self) -> Option<ZaiResult<SpeechToTextEvent>> {
        self.inner.next().await
    }
}

impl Stream for SpeechToTextStream {
    type Item = ZaiResult<SpeechToTextEvent>;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(context)
    }
}

/// Type-state request builder for speech-to-text transcription.
///
/// Exactly one input must be configured with
/// [`with_file_path`](Self::with_file_path) or
/// [`with_file_base64`](Self::with_file_base64). Non-streaming requests use
/// [`send_via`](Self::send_via); [`enable_stream`](Self::enable_stream)
/// switches the builder to the streaming-only [`stream_via`](Self::stream_via)
/// API.
pub struct AudioToTextRequest<N, S = StreamOff>
where
    N: AudioToText,
    S: StreamState,
{
    body: AudioToTextBody<N>,
    file_path: Option<PathBuf>,
    file_base64: Option<String>,
    _stream: PhantomData<S>,
}

impl<N> AudioToTextRequest<N, StreamOff>
where
    N: AudioToText,
{
    /// Create a non-streaming ASR request for the selected model.
    pub fn new(model: N) -> Self {
        Self {
            body: AudioToTextBody::new(model),
            file_path: None,
            file_base64: None,
            _stream: PhantomData,
        }
    }

    /// Switch to the typed SSE response API and serialize `stream=true`.
    pub fn enable_stream(self) -> AudioToTextRequest<N, StreamOn> {
        AudioToTextRequest {
            body: self.body.with_stream(true),
            file_path: self.file_path,
            file_base64: self.file_base64,
            _stream: PhantomData,
        }
    }

    /// Submit a non-streaming request and decode its required response fields.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<AudioToTextResponse> {
        let factory = self.build_multipart().await?;
        let route = crate::client::routes::AUDIO_TRANSCRIBE;
        let url = client.endpoints().resolve_route(route, &[])?;
        client
            .send_multipart::<AudioToTextResponse>(route.method(), url, &factory)
            .await
    }
}

impl<N> AudioToTextRequest<N, StreamOn>
where
    N: AudioToText,
{
    /// Return to the non-streaming response API and serialize `stream=false`.
    pub fn disable_stream(self) -> AudioToTextRequest<N, StreamOff> {
        AudioToTextRequest {
            body: self.body.with_stream(false),
            file_path: self.file_path,
            file_base64: self.file_base64,
            _stream: PhantomData,
        }
    }

    /// Submit the multipart request and decode typed transcription SSE events.
    ///
    /// The streaming POST is never retried or redirected. A missing `[DONE]`,
    /// malformed event, in-band business error, oversized event, or idle
    /// timeout is returned once and then terminates the stream.
    pub async fn stream_via(&self, client: &ZaiClient) -> ZaiResult<SpeechToTextStream> {
        let factory = self.build_multipart().await?;
        let route = crate::client::routes::AUDIO_TRANSCRIBE;
        let url = client.endpoints().resolve_route(route, &[])?;
        let raw = client
            .send_sse_multipart(route.method(), url, &factory)
            .await?;
        let inner = crate::model::sse_parser::decode_required_done_stream(raw, |payload| {
            serde_json::from_slice::<SpeechToTextEvent>(payload).map_err(ZaiError::from)
        });
        Ok(SpeechToTextStream { inner })
    }
}

impl<N, S> AudioToTextRequest<N, S>
where
    N: AudioToText,
    S: StreamState,
{
    /// Borrow the multipart metadata body.
    pub fn body(&self) -> &AudioToTextBody<N> {
        &self.body
    }

    /// Borrow the configured local input path, if any.
    pub fn file_path(&self) -> Option<&Path> {
        self.file_path.as_deref()
    }

    /// Whether a base64 input is configured.
    ///
    /// The encoded audio itself is intentionally not exposed by an accessor or
    /// by `Debug` output.
    pub fn has_file_base64(&self) -> bool {
        self.file_base64.is_some()
    }

    /// Select a local WAV or MP3 file. Supplying base64 as well is an error;
    /// setters do not silently override one another.
    pub fn with_file_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.file_path = Some(path.into());
        self
    }

    /// Select standard-base64 encoded WAV or MP3 bytes. Supplying a local file
    /// as well is an error.
    pub fn with_file_base64(mut self, encoded: impl Into<String>) -> Self {
        self.file_base64 = Some(encoded.into());
        self
    }

    /// Set optional prior-transcript context. The upstream 8,000-character
    /// recommendation remains advisory and is not hard-rejected by the SDK.
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.body = self.body.with_prompt(prompt);
        self
    }

    /// Set at most 100 non-blank hot words.
    pub fn with_hotwords(mut self, hotwords: Vec<String>) -> ZaiResult<Self> {
        self.body = self.body.with_hotwords(hotwords)?;
        Ok(self)
    }

    /// Set the client request identifier (`6..=64` characters).
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.body = self.body.with_request_id(request_id);
        self
    }

    /// Set the end-user identifier (`6..=128` characters).
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.body = self.body.with_user_id(user_id);
        self
    }

    /// Validate metadata and the selected input without network I/O.
    pub fn validate(&self) -> ZaiResult<()> {
        self.validate_common()?;
        match self.input()? {
            AudioInputRef::File(path) => validate_local_file(path),
            AudioInputRef::Base64(encoded) => validate_base64_audio(encoded),
        }
    }

    fn validate_common(&self) -> ZaiResult<()> {
        self.body.validate().map_err(ZaiError::from)?;
        let _ = self.input()?;
        Ok(())
    }

    fn input(&self) -> ZaiResult<AudioInputRef<'_>> {
        match (self.file_path.as_deref(), self.file_base64.as_deref()) {
            (Some(path), None) => Ok(AudioInputRef::File(path)),
            (None, Some(encoded)) => Ok(AudioInputRef::Base64(encoded)),
            (Some(_), Some(_)) => Err(validation_error(
                "ASR input requires file XOR file_base64; both were provided",
            )),
            (None, None) => Err(validation_error(
                "ASR input requires exactly one of file or file_base64",
            )),
        }
    }

    async fn build_multipart(
        &self,
    ) -> ZaiResult<crate::client::transport::multipart::MultipartBodyFactory> {
        self.validate_common()?;
        let mut factory = crate::client::transport::multipart::MultipartBodyFactory::new()
            .field("model", N::NAME)?
            .field("stream", self.body.stream.to_string())?;
        if let Some(prompt) = &self.body.prompt {
            factory = factory.field("prompt", prompt.clone())?;
        }
        for hotword in &self.body.hotwords {
            factory = factory.field("hotwords", hotword.clone())?;
        }
        if let Some(request_id) = &self.body.request_id {
            factory = factory.field("request_id", request_id.clone())?;
        }
        if let Some(user_id) = &self.body.user_id {
            factory = factory.field("user_id", user_id.clone())?;
        }

        match self.input()? {
            AudioInputRef::File(path) => {
                let mime = audio_mime_type(path)?;
                let part = crate::client::transport::multipart::FilePart::from_path_async(path)
                    .await?
                    .with_content_type(mime)?;
                if part.len() > ASR_FILE_MAX_BYTES {
                    return Err(file_error(
                        codes::SDK_FILE_TOO_LARGE,
                        "ASR audio exceeds the 25 MiB limit",
                    ));
                }
                factory.file_named("file", part)
            },
            AudioInputRef::Base64(encoded) => {
                validate_base64_audio(encoded)?;
                factory.field_with_total_limit(
                    "file_base64",
                    encoded.to_owned(),
                    ASR_BASE64_MULTIPART_LIMIT,
                )
            },
        }
    }
}

fn validate_local_file(path: &Path) -> ZaiResult<()> {
    let _ = audio_mime_type(path)?;
    let metadata = std::fs::symlink_metadata(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            file_error(codes::SDK_FILE_NOT_FOUND, "ASR file_path was not found")
        } else {
            ZaiError::from(error)
        }
    })?;
    if !metadata.is_file() {
        return Err(file_error(
            codes::SDK_FILE_NOT_FOUND,
            "ASR file_path must be a regular, non-symlink file",
        ));
    }
    if metadata.len() > ASR_FILE_MAX_BYTES {
        return Err(file_error(
            codes::SDK_FILE_TOO_LARGE,
            "ASR audio exceeds the 25 MiB limit",
        ));
    }
    Ok(())
}

fn validate_base64_audio(encoded: &str) -> ZaiResult<()> {
    if encoded.len() as u64 > ASR_BASE64_MAX_BYTES {
        return Err(file_error(
            codes::SDK_FILE_TOO_LARGE,
            "ASR base64 audio exceeds the 25 MiB decoded limit",
        ));
    }
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| validation_error("file_base64 must use valid standard base64"))?;
    if decoded.len() as u64 > ASR_FILE_MAX_BYTES {
        return Err(file_error(
            codes::SDK_FILE_TOO_LARGE,
            "ASR base64 audio exceeds the 25 MiB decoded limit",
        ));
    }
    if !is_wav(&decoded) && !is_mp3(&decoded) {
        return Err(file_error(
            codes::SDK_FILE_TYPE_UNSUPPORTED,
            "file_base64 must contain WAV or MP3 audio",
        ));
    }
    Ok(())
}

fn is_wav(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WAVE"
}

fn is_mp3(bytes: &[u8]) -> bool {
    bytes.starts_with(b"ID3") || (bytes.len() >= 2 && bytes[0] == 0xff && bytes[1] & 0xe0 == 0xe0)
}

fn audio_mime_type(path: &Path) -> ZaiResult<&'static str> {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("wav") => Ok("audio/wav"),
        Some("mp3") => Ok("audio/mpeg"),
        _ => Err(file_error(
            codes::SDK_FILE_TYPE_UNSUPPORTED,
            "ASR file_path must use a .wav or .mp3 extension",
        )),
    }
}

fn validation_error(message: impl Into<String>) -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_VALIDATION,
        message: message.into(),
    }
}

fn file_error(code: u16, message: impl Into<String>) -> ZaiError {
    ZaiError::FileError {
        code,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::audio_to_text::GlmAsr;

    fn wav_base64() -> String {
        base64::engine::general_purpose::STANDARD.encode(b"RIFF\x04\0\0\0WAVE")
    }

    #[test]
    fn input_is_exactly_one_of_file_or_base64() {
        let neither = AudioToTextRequest::new(GlmAsr {});
        assert_eq!(
            neither.validate().unwrap_err().code(),
            Some(codes::SDK_VALIDATION)
        );

        let both = AudioToTextRequest::new(GlmAsr {})
            .with_file_path("audio.wav")
            .with_file_base64(wav_base64());
        let error = both.validate().unwrap_err();
        assert_eq!(error.code(), Some(codes::SDK_VALIDATION));
        assert!(error.message().contains("XOR"));
    }

    #[test]
    fn base64_requires_wav_or_mp3_and_obeys_decoded_limit() {
        assert!(
            AudioToTextRequest::new(GlmAsr {})
                .with_file_base64(wav_base64())
                .validate()
                .is_ok()
        );
        assert!(
            AudioToTextRequest::new(GlmAsr {})
                .with_file_base64(base64::engine::general_purpose::STANDARD.encode(b"not audio"))
                .validate()
                .is_err()
        );
        assert!(
            AudioToTextRequest::new(GlmAsr {})
                .with_file_base64("%%%")
                .validate()
                .is_err()
        );
    }

    #[test]
    fn stream_type_state_sets_the_wire_flag() {
        let request = AudioToTextRequest::new(GlmAsr {}).with_file_base64(wav_base64());
        assert!(!request.body().is_streaming());
        let request = request.enable_stream();
        assert!(request.body().is_streaming());
        assert!(!request.disable_stream().body().is_streaming());
    }

    #[test]
    fn local_input_accepts_only_wav_or_mp3_up_to_25_mib() {
        let directory = tempfile::tempdir().unwrap();
        let wav = directory.path().join("audio.WAV");
        std::fs::write(&wav, b"small").unwrap();
        assert!(
            AudioToTextRequest::new(GlmAsr {})
                .with_file_path(&wav)
                .validate()
                .is_ok()
        );

        let unsupported = directory.path().join("audio.flac");
        std::fs::write(&unsupported, b"small").unwrap();
        assert_eq!(
            AudioToTextRequest::new(GlmAsr {})
                .with_file_path(&unsupported)
                .validate()
                .unwrap_err()
                .code(),
            Some(codes::SDK_FILE_TYPE_UNSUPPORTED)
        );

        let oversized = directory.path().join("large.mp3");
        let file = std::fs::File::create(&oversized).unwrap();
        file.set_len(ASR_FILE_MAX_BYTES + 1).unwrap();
        assert_eq!(
            AudioToTextRequest::new(GlmAsr {})
                .with_file_path(&oversized)
                .validate()
                .unwrap_err()
                .code(),
            Some(codes::SDK_FILE_TOO_LARGE)
        );
    }
}
