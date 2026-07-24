//! End-to-end tests for the realtime WebSocket session against a scripted
//! local mock server.
//!
//! Every test runs entirely on loopback: the mock server captures the upgrade
//! request and inbound client frames while pushing a scripted event sequence,
//! so handshake auth, typed event decoding, error mapping and close semantics
//! are exercised without touching the real Zhipu realtime API.
#![cfg(feature = "realtime")]

mod support;

use std::time::Duration;

use base64::Engine as _;
use futures_util::{Stream, StreamExt as _};
use hmac::{Hmac, KeyInit, Mac};
use serde_json::{Value, json};
use sha2::Sha256;
use support::ws_server::{CapturedFrame, ScriptedFrame, WsTestServer};
use zai_rs::{
    ZaiResult,
    client::EndpointConfig,
    model::GLM_realtime_flash,
    realtime::{RealtimeClient, RealtimeSession, ServerEvent},
};

/// Well-formed test credential in the `<id>.<secret>` shape the JWT path
/// requires; it never leaves the loopback mock server.
const TEST_KEY: &str = "test.12345678901234567890";
/// The secret half of [`TEST_KEY`], used to verify JWT signatures locally.
const TEST_KEY_SECRET: &str = "12345678901234567890";
/// Upper bound for any single wait so a stalled session fails fast instead of
/// hanging the test binary.
const READ_TIMEOUT: Duration = Duration::from_secs(5);

/// Build a client pointed at the mock server. Plaintext `ws://` is accepted
/// because the endpoint validator permits insecure transport for loopback
/// hosts only (`EndpointConfigBuilder::build(true)`).
fn client_for(server: &WsTestServer) -> RealtimeClient {
    let endpoints = EndpointConfig::builder()
        .realtime(format!("{}/realtime", server.url))
        .build(true)
        .unwrap();
    RealtimeClient::new(TEST_KEY).with_endpoint_config(endpoints)
}

/// Open a session against a mock server running `script`.
async fn open_session(script: Vec<ScriptedFrame>) -> (WsTestServer, RealtimeSession) {
    let server = WsTestServer::start(script).await;
    let session = client_for(&server)
        .session(GLM_realtime_flash {})
        .build()
        .await
        .unwrap();
    (server, session)
}

/// Pull the next item from a session stream, unwrapping the timeout and the
/// stream result so a regression fails fast with a clear panic.
async fn next_or_timeout<T>(
    stream: &mut (impl Stream<Item = ZaiResult<T>> + Unpin),
    what: &str,
) -> T {
    tokio::time::timeout(READ_TIMEOUT, stream.next())
        .await
        .unwrap_or_else(|_| panic!("timed out waiting for {what}"))
        .unwrap_or_else(|| panic!("{what} stream ended unexpectedly"))
        .unwrap_or_else(|error| panic!("{what} stream returned an error: {error}"))
}

/// Parse a captured client frame as JSON.
fn frame_json(frame: &CapturedFrame) -> Value {
    serde_json::from_str(frame.as_text().expect("expected a text frame")).unwrap()
}

/// Decode one base64url (unpadded) JWT segment.
fn base64url_decode(segment: &str) -> Vec<u8> {
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(segment)
        .unwrap()
}

#[tokio::test]
async fn bearer_handshake_sends_raw_key_and_typed_session_update() {
    let server = WsTestServer::start(vec![ScriptedFrame::json(json!({
        "type": "session.created",
        "session": {
            "id": "sess_1",
            "object": "realtime.session",
            "model": "glm-realtime-flash",
        }
    }))])
    .await;
    let session = client_for(&server)
        .session(GLM_realtime_flash {})
        .build()
        .await
        .unwrap();

    // The scripted event arrives on the typed event stream.
    let mut events = session.events();
    let event = next_or_timeout(&mut events, "session.created").await;
    assert!(
        matches!(&event, ServerEvent::SessionCreated { session } if session.id.as_deref() == Some("sess_1")),
        "unexpected event: {event:?}"
    );
    drop(events);

    // The server saw the upgrade request carrying the raw key as the Bearer
    // token, sent to the overridden endpoint path.
    let handshakes = server.handshakes();
    assert_eq!(handshakes.len(), 1);
    assert_eq!(handshakes[0].path, "/realtime");
    let expected = format!("Bearer {TEST_KEY}");
    assert_eq!(
        handshakes[0].authorization.as_deref(),
        Some(expected.as_str())
    );

    // The first client frame is the typed session.update naming the model.
    let frames = server.wait_for_frames(1).await;
    let init = frame_json(&frames[0]);
    assert_eq!(init["type"], "session.update");
    assert_eq!(init["session"]["model"], "glm-realtime-flash");

    session.close().await.unwrap();
    server.shutdown().await;
}

#[tokio::test]
async fn jwt_handshake_sends_verifiable_token_without_leaking_key() {
    let server = WsTestServer::start(Vec::new()).await;
    let session = client_for(&server)
        .with_jwt(600)
        .session(GLM_realtime_flash {})
        .build()
        .await
        .unwrap();

    // The capture callback runs before the upgrade response is flushed, so
    // the handshake is already recorded once `build` returns.
    let handshakes = server.handshakes();
    assert_eq!(handshakes.len(), 1);
    let handshake = &handshakes[0];
    let authorization = handshake.authorization.as_deref().unwrap();
    let token = authorization.strip_prefix("Bearer ").unwrap();

    // Neither the raw key nor its secret half may appear on the wire...
    assert_ne!(token, TEST_KEY);
    assert!(!token.contains(TEST_KEY_SECRET));
    for (name, value) in &handshake.headers {
        assert!(
            !value.contains(TEST_KEY_SECRET),
            "api key secret leaked into the {name} header"
        );
    }

    // ...instead the token is a three-segment JWT with the GLM header shape.
    let segments: Vec<&str> = token.split('.').collect();
    assert_eq!(segments.len(), 3, "JWT must have header.payload.signature");
    let header: Value = serde_json::from_slice(&base64url_decode(segments[0])).unwrap();
    assert_eq!(header["alg"], "HS256");
    assert_eq!(header["sign_type"], "SIGN");

    let payload: Value = serde_json::from_slice(&base64url_decode(segments[1])).unwrap();
    assert_eq!(payload["api_key"], "test");
    let now = chrono::Utc::now().timestamp();
    let exp = payload["exp"].as_i64().unwrap();
    assert!(
        now < exp && exp <= now + 600,
        "exp {exp} is not within the 600s ttl"
    );
    assert!(payload["timestamp"].is_number());

    // The signature verifies under the secret half of the API key (HS256).
    let mut mac = Hmac::<Sha256>::new_from_slice(TEST_KEY_SECRET.as_bytes()).unwrap();
    mac.update(format!("{}.{}", segments[0], segments[1]).as_bytes());
    let expected = mac.finalize().into_bytes();
    assert_eq!(base64url_decode(segments[2]), expected.as_slice());

    session.close().await.unwrap();
    server.shutdown().await;
}

#[tokio::test]
async fn client_events_and_server_events_round_trip() {
    let pcm = [0x01u8, 0x02, 0x03, 0x04];
    let script = vec![
        ScriptedFrame::json(json!({
            "type": "session.updated",
            "session": {
                "input_audio_format": "wav",
                "output_audio_format": "pcm",
                "turn_detection": {"type": "client_vad"},
                "beta_fields": {"chat_mode": "audio"},
            }
        })),
        ScriptedFrame::json(json!({
            "type": "response.text.delta",
            "response_id": "resp_1",
            "item_id": "item_1",
            "output_index": 0,
            "content_index": 0,
            "delta": "你好",
        })),
        ScriptedFrame::json(json!({
            "type": "response.text.done",
            "response_id": "resp_1",
            "item_id": "item_1",
            "text": "你好",
        })),
        ScriptedFrame::json(json!({
            "type": "response.audio.delta",
            "response_id": "resp_1",
            "item_id": "item_1",
            "output_index": 0,
            "content_index": 0,
            "delta": base64::engine::general_purpose::STANDARD.encode(pcm),
        })),
        ScriptedFrame::json(json!({
            "type": "conversation.item.input_audio_transcription.completed",
            "item_id": "item_9",
            "content_index": 0,
            "transcript": "转写文本",
        })),
    ];
    let (server, session) = open_session(script).await;

    session.send_text("你好").await.unwrap();
    session.create_response().await.unwrap();

    // Server events arrive typed and in wire order; audio deltas are decoded
    // onto the dedicated audio stream rather than the event stream.
    let mut events = session.events();
    let mut audio = session.audio_stream();

    let updated = next_or_timeout(&mut events, "session.updated").await;
    assert!(matches!(updated, ServerEvent::SessionUpdated { .. }));

    let delta = next_or_timeout(&mut events, "response.text.delta").await;
    assert!(matches!(delta, ServerEvent::ResponseTextDelta {
            response_id,
            item_id,
            output_index,
            content_index,
            delta,
        } if response_id == "resp_1" && item_id == "item_1"
            && output_index == Some(0) && content_index == Some(0) && delta == "你好"));

    let done = next_or_timeout(&mut events, "response.text.done").await;
    assert!(
        matches!(done, ServerEvent::ResponseTextDone { text: Some(text), .. } if text == "你好")
    );

    let transcript = next_or_timeout(&mut events, "transcription.completed").await;
    assert!(
        matches!(transcript, ServerEvent::InputAudioTranscriptionCompleted {
            item_id,
            transcript,
            ..
        } if item_id == "item_9" && transcript == "转写文本")
    );

    let chunk = next_or_timeout(&mut audio, "audio chunk").await;
    assert_eq!(chunk.response_id, "resp_1");
    assert_eq!(chunk.item_id, "item_1");
    assert_eq!(chunk.output_index, Some(0));
    assert_eq!(chunk.content_index, Some(0));
    assert_eq!(chunk.data.as_ref(), pcm.as_slice());
    drop(events);
    drop(audio);

    // The client's own commands reached the server, in send order.
    let frames = server.wait_for_frames(3).await;
    let messages: Vec<Value> = frames.iter().map(frame_json).collect();
    let types: Vec<&str> = messages
        .iter()
        .map(|message| message["type"].as_str().unwrap())
        .collect();
    assert_eq!(
        types,
        [
            "session.update",
            "conversation.item.create",
            "response.create"
        ]
    );
    assert_eq!(messages[1]["item"]["role"], "user");
    assert_eq!(messages[1]["item"]["content"][0]["type"], "input_text");
    assert_eq!(messages[1]["item"]["content"][0]["text"], "你好");

    session.close().await.unwrap();
    server.shutdown().await;
}

#[tokio::test]
async fn burst_pings_are_coalesced_without_closing_the_session() {
    let pings = (0_u8..=8).map(|value| vec![value]).collect();
    let (server, session) = open_session(vec![
        ScriptedFrame::PingBurst(pings),
        ScriptedFrame::json(json!({
            "type": "response.text.delta",
            "response_id": "resp_ping",
            "item_id": "item_ping",
            "delta": "alive",
        })),
    ])
    .await;

    let mut events = session.events();
    let event = next_or_timeout(&mut events, "event after Ping burst").await;
    assert!(
        matches!(event, ServerEvent::ResponseTextDelta { delta, .. } if delta == "alive"),
        "session closed while processing a legal Ping burst"
    );
    drop(events);

    session.send_text("still alive").await.unwrap();
    let frames = server.wait_for_frames(2).await;
    assert_eq!(frame_json(&frames[1])["type"], "conversation.item.create");

    session.close().await.unwrap();
    server.shutdown().await;
}

#[tokio::test]
async fn server_error_event_maps_to_typed_error_and_session_survives() {
    let (server, session) = open_session(vec![ScriptedFrame::json(json!({
        "type": "error",
        "error": {
            "type": "invalid_request_error",
            "code": "invalid_event",
            "message": "The 'type' field is missing.",
        }
    }))])
    .await;

    let mut events = session.events();
    let event = next_or_timeout(&mut events, "error event").await;
    match event {
        ServerEvent::Error { error } => {
            assert_eq!(error.type_, "invalid_request_error");
            assert_eq!(error.code.as_deref(), Some("invalid_event"));
            assert_eq!(error.message, "The 'type' field is missing.");
        },
        other => panic!("expected a typed error event, got {other:?}"),
    }
    drop(events);

    // Protocol error events are recoverable: the session keeps pumping client
    // commands after one is received.
    session.send_text("still alive").await.unwrap();
    let frames = server.wait_for_frames(2).await;
    assert_eq!(frame_json(&frames[1])["type"], "conversation.item.create");

    session.close().await.unwrap();
    server.shutdown().await;
}

#[tokio::test]
async fn malformed_server_frame_closes_session_with_protocol_error() {
    let (server, session) = open_session(vec![ScriptedFrame::Text("not json".into())]).await;

    let mut events = session.events();
    let error = tokio::time::timeout(READ_TIMEOUT, events.next())
        .await
        .expect("timed out waiting for the protocol error")
        .expect("event stream ended without surfacing the protocol error")
        .expect_err("a malformed frame must surface as a stream error");
    assert!(error.message().contains("malformed realtime server event"));
    drop(events);

    // The failed background loop propagates its error to the close path.
    assert!(session.close().await.is_err());
    server.shutdown().await;
}

#[tokio::test]
async fn unexpected_binary_frame_closes_session_with_protocol_error() {
    let (server, session) = open_session(vec![ScriptedFrame::Binary(vec![0x00, 0x01])]).await;

    let mut events = session.events();
    let error = tokio::time::timeout(READ_TIMEOUT, events.next())
        .await
        .expect("timed out waiting for the protocol error")
        .expect("event stream ended without surfacing the protocol error")
        .expect_err("a binary frame must surface as a stream error");
    assert!(error.message().contains("unexpected binary frame"));
    drop(events);

    assert!(session.close().await.is_err());
    server.shutdown().await;
}

#[tokio::test]
async fn server_initiated_close_ends_the_event_stream_cleanly() {
    let (server, session) = open_session(vec![
        ScriptedFrame::json(json!({
            "type": "response.text.delta",
            "response_id": "resp_1",
            "item_id": "item_1",
            "delta": "再见",
        })),
        ScriptedFrame::Close,
    ])
    .await;

    let mut events = session.events();
    let delta = next_or_timeout(&mut events, "response.text.delta").await;
    assert!(matches!(delta, ServerEvent::ResponseTextDelta { delta, .. } if delta == "再见"));

    // A peer close completes the loop with `Ok(())`, so the stream ends
    // without surfacing an error item.
    let end = tokio::time::timeout(READ_TIMEOUT, events.next())
        .await
        .expect("timed out waiting for the event stream to end");
    assert!(
        end.is_none(),
        "event stream must end cleanly after a peer close"
    );
    drop(events);

    // Joining the already-finished loop reports its clean result.
    session.close().await.unwrap();
    server.shutdown().await;
}
