use std::{
    io::Read as _,
    marker::PhantomData,
    path::{Path, PathBuf},
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures_util::{Stream, StreamExt};
use validator::Validate;

use super::{
    super::traits::{AudioToText, StreamOff, StreamOn, StreamState},
    request::AudioToTextBody,
    response::{AudioToTextResponse, SpeechToTextEvent},
};
use crate::{
    ZaiError, ZaiResult,
    client::{ZaiClient, error::codes, validation::invalid},
};

const ASR_FILE_MAX_BYTES: u64 = 25 * 1024 * 1024;
const ASR_BASE64_MAX_BYTES: u64 = ASR_FILE_MAX_BYTES.div_ceil(3) * 4;
const ASR_BASE64_MULTIPART_LIMIT: u64 =
    ASR_BASE64_MAX_BYTES + crate::client::transport::limits::MULTIPART_FIELD_BYTES_MAX + 64;

enum AudioInputRef<'a> {
    File(&'a Path),
    Base64(&'a Bytes),
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
    file_base64: Option<Bytes>,
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
        client
            .operation(route)
            .send_multipart::<AudioToTextResponse>(&factory)
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
    /// The handshake accepts only an unranged `200 OK` with
    /// `text/event-stream`, and the streaming POST is never retried or
    /// redirected. A missing `[DONE]`, malformed event, in-band business error,
    /// oversized event, or idle timeout is returned once and then terminates
    /// the stream.
    pub async fn stream_via(&self, client: &ZaiClient) -> ZaiResult<SpeechToTextStream> {
        let factory = self.build_multipart().await?;
        let route = crate::client::routes::AUDIO_TRANSCRIBE;
        let raw = client.operation(route).send_sse_multipart(&factory).await?;
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
        self.file_base64 = Some(Bytes::from(encoded.into()));
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
        match (self.file_path.as_deref(), self.file_base64.as_ref()) {
            (Some(path), None) => Ok(AudioInputRef::File(path)),
            (None, Some(encoded)) => Ok(AudioInputRef::Base64(encoded)),
            (Some(_), Some(_)) => Err(invalid(
                "ASR input requires file XOR file_base64; both were provided",
            )),
            (None, None) => Err(invalid(
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
                factory.field_bytes_with_total_limit(
                    "file_base64",
                    encoded.clone(),
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

fn validate_base64_audio(encoded: &[u8]) -> ZaiResult<()> {
    if encoded.len() as u64 > ASR_BASE64_MAX_BYTES {
        return Err(file_error(
            codes::SDK_FILE_TOO_LARGE,
            "ASR base64 audio exceeds the 25 MiB decoded limit",
        ));
    }

    // Decode the complete input through a fixed-size buffer. Reading to EOF is
    // intentional: a valid audio prefix must not hide malformed base64 later
    // in the field, and the error precedence remains identical to a one-shot
    // `STANDARD.decode`.
    let mut decoder =
        base64::read::DecoderReader::new(encoded, &base64::engine::general_purpose::STANDARD);
    let mut scratch = [0_u8; 8 * 1024];
    let mut signature = [0_u8; 12];
    let mut signature_len = 0_usize;
    let mut decoded_len = 0_u64;
    loop {
        let read = decoder
            .read(&mut scratch)
            .map_err(|_| invalid("file_base64 must use valid standard base64"))?;
        if read == 0 {
            break;
        }

        if signature_len < signature.len() {
            let copied = (signature.len() - signature_len).min(read);
            signature[signature_len..signature_len + copied].copy_from_slice(&scratch[..copied]);
            signature_len += copied;
        }
        decoded_len = decoded_len.saturating_add(read as u64);
    }

    if decoded_len > ASR_FILE_MAX_BYTES {
        return Err(file_error(
            codes::SDK_FILE_TOO_LARGE,
            "ASR base64 audio exceeds the 25 MiB decoded limit",
        ));
    }
    let signature = &signature[..signature_len];
    if !is_wav(signature) && !is_mp3(signature) {
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

fn file_error(code: u16, message: impl Into<String>) -> ZaiError {
    ZaiError::FileError {
        code,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use base64::Engine as _;

    use super::*;
    use crate::model::audio_to_text::GlmAsr;

    fn wav_base64() -> String {
        base64::engine::general_purpose::STANDARD.encode(b"RIFF\x04\0\0\0WAVE")
    }

    fn wav_bytes(len: usize) -> Vec<u8> {
        assert!(len >= 12);
        let mut bytes = vec![0_u8; len];
        bytes[..4].copy_from_slice(b"RIFF");
        bytes[8..12].copy_from_slice(b"WAVE");
        bytes
    }

    fn assert_base64_error(encoded: impl Into<String>, code: u16, message: &str) {
        let error = AudioToTextRequest::new(GlmAsr {})
            .with_file_base64(encoded)
            .validate()
            .unwrap_err();
        assert_eq!(error.code(), Some(code));
        assert_eq!(error.message(), message);
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
    fn base64_accepts_wav_and_both_documented_mp3_signatures() {
        for audio in [
            b"RIFF\x04\0\0\0WAVE".as_slice(),
            b"ID3fake-mp3".as_slice(),
            b"\xff\xe0fake-mp3".as_slice(),
        ] {
            assert!(
                AudioToTextRequest::new(GlmAsr {})
                    .with_file_base64(base64::engine::general_purpose::STANDARD.encode(audio))
                    .validate()
                    .is_ok()
            );
        }

        assert_base64_error(
            base64::engine::general_purpose::STANDARD.encode(b"not audio"),
            codes::SDK_FILE_TYPE_UNSUPPORTED,
            "file_base64 must contain WAV or MP3 audio",
        );
    }

    #[test]
    fn base64_streaming_validation_preserves_standard_padding_and_malformed_semantics() {
        let padded = base64::engine::general_purpose::STANDARD.encode(b"RIFF\x04\0\0\0WAVEx");
        assert!(padded.ends_with("=="));

        let mut non_canonical_trailing_bits = padded.as_bytes().to_vec();
        let last_symbol = non_canonical_trailing_bits.len() - 3;
        non_canonical_trailing_bits[last_symbol] = b'B';
        let non_canonical_trailing_bits = String::from_utf8(non_canonical_trailing_bits).unwrap();

        let mut url_safe_audio = wav_bytes(15);
        url_safe_audio[12..].copy_from_slice(&[0xff, 0xff, 0xff]);
        let url_safe = base64::engine::general_purpose::URL_SAFE.encode(url_safe_audio);
        assert!(url_safe.contains('_'));

        let malformed = [
            "%%%".to_string(),
            format!("{}%", wav_base64()),
            format!("{}\n", wav_base64()),
            format!("{}=", wav_base64()),
            padded.trim_end_matches('=').to_string(),
            format!("{padded}AAAA"),
            non_canonical_trailing_bits,
            url_safe,
        ];
        for encoded in malformed {
            assert!(
                base64::engine::general_purpose::STANDARD
                    .decode(&encoded)
                    .is_err(),
                "malformed regression fixture unexpectedly decoded: {encoded}"
            );
            assert_base64_error(
                encoded,
                codes::SDK_VALIDATION,
                "file_base64 must use valid standard base64",
            );
        }
    }

    #[test]
    fn base64_valid_magic_does_not_hide_a_malformed_tail_after_many_decoder_buffers() {
        let mut encoded = base64::engine::general_purpose::STANDARD
            .encode(wav_bytes(32 * 1024))
            .into_bytes();
        *encoded.last_mut().unwrap() = b'%';
        assert_base64_error(
            String::from_utf8(encoded).unwrap(),
            codes::SDK_VALIDATION,
            "file_base64 must use valid standard base64",
        );
    }

    #[test]
    fn base64_decoded_size_limit_is_exact() {
        let mut audio = wav_bytes(ASR_FILE_MAX_BYTES as usize);
        let at_limit = base64::engine::general_purpose::STANDARD.encode(&audio);
        assert_eq!(at_limit.len() as u64, ASR_BASE64_MAX_BYTES);
        assert!(
            AudioToTextRequest::new(GlmAsr {})
                .with_file_base64(at_limit)
                .validate()
                .is_ok()
        );

        // 25 MiB is congruent to one modulo three, so one additional decoded
        // byte has the same encoded length. This proves the decoded-size check
        // is authoritative instead of relying only on the encoded preflight.
        audio.push(0);
        let over_limit = base64::engine::general_purpose::STANDARD.encode(audio);
        assert_eq!(over_limit.len() as u64, ASR_BASE64_MAX_BYTES);
        assert_base64_error(
            over_limit,
            codes::SDK_FILE_TOO_LARGE,
            "ASR base64 audio exceeds the 25 MiB decoded limit",
        );

        // Keep the existing encoded-preflight precedence: input beyond the
        // encoded cap is a size error even when the bytes are not base64.
        assert_base64_error(
            "%".repeat(ASR_BASE64_MAX_BYTES as usize + 1),
            codes::SDK_FILE_TOO_LARGE,
            "ASR base64 audio exceeds the 25 MiB decoded limit",
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

    #[tokio::test(flavor = "current_thread")]
    async fn base64_multipart_preparation_has_no_payload_sized_allocation() {
        use std::hint::black_box;

        use stats_alloc::Region;

        const CHILD_ENV: &str = "ZAI_ASR_BASE64_ALLOC_CHILD";
        const CHILD_SENTINEL: &str = "ZAI_ASR_BASE64_ALLOC_CHILD_OK";
        const METRIC_PREFIX: &str = "ZAI_ASR_BASE64_ALLOC_METRIC=";
        const EXACT_TEST_NAME: &str = concat!(
            "model::audio_to_text::data::tests::",
            "base64_multipart_preparation_has_no_payload_sized_allocation"
        );
        const MAX_ALLOCATIONS: usize = 64;
        const MAX_REALLOCATIONS: usize = 32;
        const MAX_ALLOCATED_BYTES: usize = 64 * 1024;
        const MAX_REALLOCATED_BYTES: isize = 64 * 1024;

        // The allocator counters are process-global. Run the measured region
        // in a dedicated one-test child so parallel harness activity cannot
        // create false failures on this payload-copy gate.
        if std::env::var_os(CHILD_ENV).is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    EXACT_TEST_NAME,
                    "--exact",
                    "--test-threads=1",
                    "--nocapture",
                ])
                .env(CHILD_ENV, "1")
                .output()
                .unwrap();
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(
                output.status.success(),
                "isolated ASR allocation census failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
            );
            assert_eq!(
                stdout.matches(CHILD_SENTINEL).count(),
                1,
                "ASR allocation child did not execute exactly once\nstdout:\n{stdout}\nstderr:\n{stderr}"
            );
            print!("{stdout}");
            return;
        }

        // Positive-control the instrumentation itself. Without this check the
        // measured region could report all zeroes if INSTRUMENTED_SYSTEM ever
        // stopped being installed as this test binary's global allocator.
        let positive_region = Region::new(&stats_alloc::INSTRUMENTED_SYSTEM);
        let positive_probe = vec![0_u8; 4 * 1024];
        black_box(&positive_probe);
        let positive_stats = positive_region.change();
        assert!(
            positive_stats.allocations >= 1 && positive_stats.bytes_allocated >= 4 * 1024,
            "ASR allocation census positive control was not observed: {positive_stats:?}"
        );
        drop(positive_probe);

        // Warm lazy multipart/random/runtime state before opening the census.
        let warmup = AudioToTextRequest::new(GlmAsr {}).with_file_base64(wav_base64());
        let warmup_factory = warmup.build_multipart().await.unwrap();
        let warmup_form = warmup_factory.build().await.unwrap();
        black_box((&warmup_factory, &warmup_form));
        drop(warmup_form);
        drop(warmup_factory);
        drop(warmup);

        let audio = wav_bytes(ASR_FILE_MAX_BYTES as usize);
        let encoded = base64::engine::general_purpose::STANDARD.encode(audio);
        let encoded_pointer = encoded.as_ptr();
        let request = AudioToTextRequest::new(GlmAsr {}).with_file_base64(encoded);
        assert_eq!(
            request.file_base64.as_ref().unwrap().as_ptr(),
            encoded_pointer,
            "String-to-Bytes conversion copied the encoded payload"
        );

        let region = Region::new(&stats_alloc::INSTRUMENTED_SYSTEM);
        let factory = black_box(&request).build_multipart().await.unwrap();
        let form = factory.build().await.unwrap();
        let stream = form.into_stream();
        futures_util::pin_mut!(stream);
        let mut wire_bytes = 0_usize;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.unwrap();
            wire_bytes = wire_bytes.checked_add(chunk.len()).unwrap();
            black_box(&chunk);
        }
        black_box((&request, &factory));
        let stats = region.change();

        assert!(
            wire_bytes > ASR_BASE64_MAX_BYTES as usize,
            "drained multipart wire did not contain the complete base64 field"
        );

        assert!(
            stats.allocations <= MAX_ALLOCATIONS,
            "ASR multipart preparation allocated too often: {stats:?}"
        );
        assert!(
            stats.reallocations <= MAX_REALLOCATIONS,
            "ASR multipart preparation reallocated too often: {stats:?}"
        );
        assert!(
            stats.bytes_allocated <= MAX_ALLOCATED_BYTES,
            "ASR multipart preparation duplicated payload-sized storage: {stats:?}"
        );
        assert!(
            stats.bytes_reallocated <= MAX_REALLOCATED_BYTES,
            "ASR multipart preparation reallocated payload-sized storage: {stats:?}"
        );
        println!(
            "{METRIC_PREFIX}{{\"payload_bytes\":{},\"wire_bytes\":{wire_bytes},\"allocations\":{},\"reallocations\":{},\"bytes_allocated\":{},\"bytes_reallocated\":{}}}",
            ASR_BASE64_MAX_BYTES,
            stats.allocations,
            stats.reallocations,
            stats.bytes_allocated,
            stats.bytes_reallocated,
        );
        println!("{CHILD_SENTINEL}");
    }
}
