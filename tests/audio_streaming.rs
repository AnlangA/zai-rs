//! Frozen ASR/TTS wire and streaming contract tests.

mod support;

use base64::Engine as _;
use bytes::Bytes;
use serde_json::json;
use support::http_server::{CapturedRequest, ScriptedResponse, TestServer};
use zai_rs::{
    client::{ApiFamily, ZaiClient, error::codes},
    model::{
        audio_to_text::{AudioToTextRequest, GlmAsr},
        text_to_audio::{GlmTts, TextToAudioRequest, TtsAudioFormat, TtsEncodeFormat, Voice},
    },
};

const KEY: &str = "test.12345678901234567890";

fn client(server: &TestServer) -> ZaiClient {
    ZaiClient::builder(KEY)
        .allow_insecure_transport(true)
        .endpoint(
            ApiFamily::PaasV4,
            format!("{}/api/paas/v4", server.base_url),
        )
        .build()
        .unwrap()
}

fn only_request(server: &TestServer) -> CapturedRequest {
    let requests = server.requests();
    assert_eq!(
        requests.len(),
        1,
        "expected exactly one authenticated HTTP request"
    );
    let request = requests.into_iter().next().unwrap();
    let expected_authorization = format!("Bearer {KEY}");
    assert_eq!(
        request.authorization.as_deref(),
        Some(expected_authorization.as_str())
    );
    request
}

fn header<'a>(request: &'a CapturedRequest, name: &str) -> Option<&'a str> {
    request
        .headers
        .iter()
        .find(|(candidate, _)| candidate.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}

fn assert_multipart_field(request: &CapturedRequest, name: &str, value: &str) {
    let body = String::from_utf8_lossy(&request.body);
    assert!(body.contains(&format!("name=\"{name}\"")), "missing {name}");
    assert!(
        body.contains(&format!("name=\"{name}\"\r\n\r\n{value}\r\n")),
        "unexpected multipart value for {name}"
    );
}

fn wav_base64() -> String {
    base64::engine::general_purpose::STANDARD.encode(b"RIFF\x04\0\0\0WAVE")
}

#[tokio::test]
async fn asr_xor_violation_fails_before_network_io() {
    let server = TestServer::start(Vec::new()).await;
    let error = AudioToTextRequest::new(GlmAsr {})
        .with_file_path("audio.wav")
        .with_file_base64(wav_base64())
        .send_via(&client(&server))
        .await
        .unwrap_err();
    assert_eq!(error.code(), Some(codes::SDK_VALIDATION));
    assert!(server.requests().is_empty());
    server.shutdown().await;
}

#[tokio::test]
async fn asr_base64_nonstream_has_exact_multipart_wire_and_required_response() {
    let server = TestServer::start(vec![ScriptedResponse::json(
        200,
        json!({"id":"asr-1","model":"glm-asr-2512","text":"hello"}),
    )])
    .await;
    let response = AudioToTextRequest::new(GlmAsr {})
        .with_file_base64(wav_base64())
        .with_prompt("prior transcript")
        .with_hotwords(vec!["alpha".into(), "beta".into()])
        .unwrap()
        .with_request_id("req-01")
        .with_user_id("user-01")
        .send_via(&client(&server))
        .await
        .unwrap();

    assert_eq!(response.id, "asr-1");
    assert_eq!(response.model, "glm-asr-2512");
    assert_eq!(response.text, "hello");
    let request = only_request(&server);
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/api/paas/v4/audio/transcriptions");
    assert_eq!(header(&request, "accept"), Some("application/json"));
    assert!(
        header(&request, "content-type")
            .is_some_and(|value| value.starts_with("multipart/form-data; boundary="))
    );
    assert_multipart_field(&request, "model", "glm-asr-2512");
    assert_multipart_field(&request, "stream", "false");
    assert_multipart_field(&request, "file_base64", &wav_base64());
    assert_multipart_field(&request, "prompt", "prior transcript");
    assert_multipart_field(&request, "hotwords", "alpha");
    assert_multipart_field(&request, "hotwords", "beta");
    assert_multipart_field(&request, "request_id", "req-01");
    assert_multipart_field(&request, "user_id", "user-01");
    let body = String::from_utf8_lossy(&request.body);
    assert_eq!(body.matches("name=\"hotwords\"").count(), 2);
    assert!(!body.contains("name=\"file\""));
    server.shutdown().await;
}

#[tokio::test]
async fn asr_local_file_streams_typed_events_with_file_mime_and_done() {
    let body = concat!(
        "data: {\"id\":\"asr-1\",\"model\":\"glm-asr-2512\",\"type\":\"transcript.text.delta\",\"delta\":\"hel\"}\n\n",
        "data: {\"id\":\"asr-1\",\"type\":\"transcript.text.done\",\"delta\":\"lo\"}\n\n",
        "data: [DONE]\n\n"
    );
    let server = TestServer::start(vec![ScriptedResponse::raw(
        200,
        "text/event-stream; charset=utf-8",
        body,
    )])
    .await;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("voice.mp3");
    std::fs::write(&path, b"ID3fake-mp3").unwrap();

    let mut stream = AudioToTextRequest::new(GlmAsr {})
        .with_file_path(&path)
        .enable_stream()
        .stream_via(&client(&server))
        .await
        .unwrap();
    let first = stream.next().await.unwrap().unwrap();
    let second = stream.next().await.unwrap().unwrap();
    assert_eq!(first.event_type.as_deref(), Some("transcript.text.delta"));
    assert_eq!(first.delta.as_deref(), Some("hel"));
    assert_eq!(second.event_type.as_deref(), Some("transcript.text.done"));
    assert_eq!(second.delta.as_deref(), Some("lo"));
    assert!(stream.next().await.is_none());

    let request = only_request(&server);
    assert_eq!(header(&request, "accept"), Some("text/event-stream"));
    assert_multipart_field(&request, "stream", "true");
    let body = String::from_utf8_lossy(&request.body);
    assert!(body.contains("name=\"file\"; filename=\"voice.mp3\""));
    assert!(body.contains("Content-Type: audio/mpeg"));
    server.shutdown().await;
}

#[tokio::test]
async fn asr_stream_reports_missing_done_or_malformed_event_once() {
    for body in [
        "data: {\"type\":\"transcript.text.delta\",\"delta\":\"partial\"}\n\n",
        "data: not-json\n\ndata: [DONE]\n\n",
    ] {
        let server =
            TestServer::start(vec![ScriptedResponse::raw(200, "text/event-stream", body)]).await;
        let mut stream = AudioToTextRequest::new(GlmAsr {})
            .with_file_base64(wav_base64())
            .enable_stream()
            .stream_via(&client(&server))
            .await
            .unwrap();
        if body.contains("partial") {
            assert!(stream.next().await.unwrap().is_ok());
        }
        assert!(stream.next().await.unwrap().is_err());
        assert!(stream.next().await.is_none());
        server.shutdown().await;
    }
}

#[tokio::test]
async fn tts_nonstream_has_exact_json_wire_and_audio_accept_contract() {
    let server = TestServer::start(vec![ScriptedResponse::raw(
        200,
        "audio/x-wav",
        Bytes::from_static(b"wav-bytes"),
    )])
    .await;
    let bytes = TextToAudioRequest::new(GlmTts {})
        .with_input("hello")
        .with_voice(Voice::Chuichui)
        .with_speed(1.25)
        .with_volume(2.0)
        .with_watermark_enabled(false)
        .with_response_format(TtsAudioFormat::Wav)
        .send_via(&client(&server))
        .await
        .unwrap();
    assert_eq!(bytes, Bytes::from_static(b"wav-bytes"));

    let request = only_request(&server);
    assert_eq!(request.path, "/api/paas/v4/audio/speech");
    assert_eq!(
        header(&request, "accept"),
        Some("audio/wav, audio/x-wav, audio/pcm, application/octet-stream")
    );
    let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
    assert_eq!(
        body,
        json!({
            "model": "glm-tts",
            "input": "hello",
            "voice": "chuichui",
            "speed": 1.25,
            "volume": 2.0,
            "response_format": "wav",
            "stream": false,
            "watermark_enabled": false
        })
    );
    assert!(body.get("encode_format").is_none());
    server.shutdown().await;
}

#[tokio::test]
async fn tts_stream_decodes_base64_and_hex_and_sends_pcm_wire() {
    for (encoding, encoded, expected_name) in [
        (TtsEncodeFormat::Base64, "AAEC/w==", "base64"),
        (TtsEncodeFormat::Hex, "000102fF", "hex"),
    ] {
        let server = TestServer::start(vec![ScriptedResponse::raw(
            200,
            "text/event-stream",
            format!("data: {encoded}\n\ndata: [DONE]\n\n"),
        )])
        .await;
        let mut stream = TextToAudioRequest::new(GlmTts {})
            .with_input("hello")
            .enable_stream()
            .with_encode_format(encoding)
            .stream_via(&client(&server))
            .await
            .unwrap();
        assert_eq!(
            stream.next().await.unwrap().unwrap(),
            Bytes::from_static(&[0, 1, 2, 255])
        );
        assert!(stream.next().await.is_none());

        let request = only_request(&server);
        assert_eq!(header(&request, "accept"), Some("text/event-stream"));
        let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(
            body,
            json!({
                "model": "glm-tts",
                "input": "hello",
                "voice": "tongtong",
                "response_format": "pcm",
                "stream": true,
                "encode_format": expected_name
            })
        );
        server.shutdown().await;
    }
}

#[tokio::test]
async fn tts_stream_reports_bad_encoding_and_missing_done_once() {
    for body in ["data: %%%\n\ndata: [DONE]\n\n", "data: AAEC\n\n"] {
        let server =
            TestServer::start(vec![ScriptedResponse::raw(200, "text/event-stream", body)]).await;
        let mut stream = TextToAudioRequest::new(GlmTts {})
            .with_input("hello")
            .enable_stream()
            .stream_via(&client(&server))
            .await
            .unwrap();
        if body.contains("AAEC") {
            assert_eq!(
                stream.next().await.unwrap().unwrap(),
                Bytes::from_static(&[0, 1, 2])
            );
        }
        let error = stream.next().await.unwrap().unwrap_err();
        assert_eq!(error.code(), Some(codes::SDK_IO));
        assert!(stream.next().await.is_none());
        server.shutdown().await;
    }
}

#[tokio::test]
async fn streaming_audio_requires_event_stream_mime() {
    let server = TestServer::start(vec![ScriptedResponse::raw(
        200,
        "application/json",
        json!({"data":"AAEC"}).to_string(),
    )])
    .await;
    let result = TextToAudioRequest::new(GlmTts {})
        .with_input("hello")
        .enable_stream()
        .stream_via(&client(&server))
        .await;
    let error = match result {
        Ok(_) => panic!("a streaming response must use text/event-stream"),
        Err(error) => error,
    };
    assert_eq!(error.code(), Some(codes::SDK_VALIDATION));
    assert_eq!(server.requests().len(), 1);
    server.shutdown().await;
}

#[tokio::test]
async fn streaming_post_is_neither_retried_nor_redirected() {
    for status in [307, 503] {
        let mut failure = ScriptedResponse::raw(status, "text/plain", Bytes::new());
        if status == 307 {
            failure
                .headers
                .push(("location".into(), "/api/paas/v4/audio/speech".into()));
        }
        let server = TestServer::start(vec![
            failure,
            ScriptedResponse::raw(200, "text/event-stream", "data: [DONE]\n\n"),
        ])
        .await;

        let result = TextToAudioRequest::new(GlmTts {})
            .with_input("hello")
            .enable_stream()
            .stream_via(&client(&server))
            .await;
        assert!(result.is_err());
        assert_eq!(server.requests().len(), 1);
        server.shutdown().await;
    }
}
