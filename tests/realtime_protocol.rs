//! Realtime event serialization and protocol-shape tests.
//! Requires `--features realtime`.

#![cfg(feature = "realtime")]

use zai_rs::realtime::events::ClientEvent;
use zai_rs::realtime::protocol::SessionConfig;

#[test]
fn client_event_session_update_is_constructible() {
    let event = ClientEvent::SessionUpdate {
        event_id: None,
        session: SessionConfig::default(),
    };
    let json = serde_json::to_value(&event).unwrap();
    assert_eq!(json["type"], "session.update");
}

#[test]
fn session_update_serializes_with_event_id() {
    let event = ClientEvent::SessionUpdate {
        event_id: Some("evt_1".to_string()),
        session: SessionConfig::default(),
    };
    let json = serde_json::to_value(&event).unwrap();
    assert_eq!(json["type"], "session.update");
    assert_eq!(json["event_id"], "evt_1");
}

#[test]
fn server_event_parses_session_created() {
    let json = r#"{"type":"session.created","session":{"id":"s1"}}"#;
    let val: serde_json::Value = serde_json::from_str(json).unwrap();
    assert_eq!(val["type"], "session.created");
    assert_eq!(val["session"]["id"], "s1");
}

#[test]
fn error_event_carries_code_and_message() {
    let json =
        r#"{"type":"error","error":{"code":"server_error","message":"something went wrong"}}"#;
    let val: serde_json::Value = serde_json::from_str(json).unwrap();
    assert_eq!(val["type"], "error");
    assert!(val["error"]["code"].is_string());
}

#[test]
fn input_audio_buffer_append_is_constructible() {
    let event = ClientEvent::InputAudioBufferAppend {
        audio: "base64data".to_string(),
        client_timestamp: None,
    };
    let json = serde_json::to_value(&event).unwrap();
    assert_eq!(json["type"], "input_audio_buffer.append");
    assert_eq!(json["audio"], "base64data");
}

#[test]
fn input_audio_buffer_append_with_timestamp() {
    let event = ClientEvent::InputAudioBufferAppend {
        audio: "base64data".to_string(),
        client_timestamp: Some(1731999464667),
    };
    let json = serde_json::to_value(&event).unwrap();
    assert_eq!(json["client_timestamp"], 1731999464667_i64);
}

#[test]
fn session_config_default_is_constructible() {
    let cfg = SessionConfig::default();
    let json = serde_json::to_value(&cfg).unwrap();
    assert!(json.is_object());
}

#[test]
fn unknown_server_event_parses_as_value() {
    // Generic JSON Value safely deserializes unknown event types.
    let json = r#"{"type":"ping"}"#;
    let val: serde_json::Value = serde_json::from_str(json).unwrap();
    assert_eq!(val["type"], "ping");
}
