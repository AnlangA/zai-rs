//! Coverage for transport/mod.rs and realtime/session.rs.
//! Tests the pure-logic paths (timeout policy, config builders, session config)
//! that don't require a real network connection.

use std::time::Duration;
use zai_rs::client::transport::{Clock, TimeoutPolicy, WallClock};

#[test]
fn timeout_policy_defaults() {
    let t = TimeoutPolicy::default();
    assert_eq!(t.connect, Duration::from_secs(10));
    assert_eq!(t.attempt, Duration::from_secs(60));
    assert_eq!(t.overall, Duration::from_secs(120));
    assert_eq!(t.stream_idle, Duration::from_secs(60));
}

#[test]
fn timeout_policy_custom() {
    let t = TimeoutPolicy {
        connect: Duration::from_secs(5),
        attempt: Duration::from_secs(30),
        overall: Duration::from_secs(60),
        stream_idle: Duration::from_secs(30),
    };
    assert_eq!(t.connect, Duration::from_secs(5));
}

#[test]
fn wall_clock_now_is_recent() {
    let clock = WallClock;
    let t1 = clock.now();
    std::thread::sleep(Duration::from_millis(1));
    let t2 = clock.now();
    assert!(t2 > t1);
}

#[test]
fn wall_clock_implements_clock_trait() {
    let clock: Box<dyn Clock> = Box::new(WallClock);
    let _ = clock.now();
}

// --- transport/retry deeper coverage ---
#[test]
fn retry_safety_for_all_methods() {
    use zai_rs::client::transport::retry::RetrySafety;
    assert_eq!(RetrySafety::for_method("GET"), RetrySafety::Idempotent);
    assert_eq!(RetrySafety::for_method("HEAD"), RetrySafety::Idempotent);
    assert_eq!(RetrySafety::for_method("OPTIONS"), RetrySafety::Idempotent);
    assert_eq!(RetrySafety::for_method("PUT"), RetrySafety::Idempotent);
    assert_eq!(RetrySafety::for_method("DELETE"), RetrySafety::Idempotent);
    assert_eq!(RetrySafety::for_method("POST"), RetrySafety::NonIdempotent);
    assert_eq!(RetrySafety::for_method("PATCH"), RetrySafety::NonIdempotent);
    assert_eq!(
        RetrySafety::for_method("CONNECT"),
        RetrySafety::NonIdempotent
    );
}

#[test]
fn retry_safety_effective_with_override() {
    use zai_rs::client::RetryOverride;
    use zai_rs::client::transport::retry::RetrySafety;
    assert_eq!(
        RetrySafety::NonIdempotent.effective(Some(RetryOverride::AssumeIdempotent)),
        RetrySafety::Idempotent
    );
    assert_eq!(
        RetrySafety::Idempotent.effective(Some(RetryOverride::AssumeIdempotent)),
        RetrySafety::Idempotent
    );
    assert_eq!(
        RetrySafety::NonIdempotent.effective(None),
        RetrySafety::NonIdempotent
    );
}

#[test]
fn retry_after_various_formats() {
    use zai_rs::client::transport::retry::parse_retry_after;
    assert_eq!(parse_retry_after("5"), Some(Duration::from_secs(5)));
    assert_eq!(parse_retry_after("  10  "), Some(Duration::from_secs(10)));
    assert_eq!(parse_retry_after("0"), None);
    assert_eq!(parse_retry_after(""), None);
    assert_eq!(parse_retry_after("abc"), None);
    assert_eq!(parse_retry_after("Wed, 21 Oct 2026 07:28:00 GMT"), None);
}

#[test]
fn reconcile_retry_after_takes_max() {
    use zai_rs::client::transport::retry::reconcile_retry_after;
    let computed = Duration::from_secs(2);
    assert_eq!(reconcile_retry_after(None, computed), computed);
    assert_eq!(
        reconcile_retry_after(Some(Duration::from_secs(10)), computed),
        Duration::from_secs(10)
    );
    assert_eq!(
        reconcile_retry_after(Some(Duration::from_secs(1)), computed),
        computed
    );
    assert_eq!(
        reconcile_retry_after(Some(Duration::from_secs(2)), computed),
        Duration::from_secs(2)
    );
}

// --- transport/limits deeper ---
#[test]
fn limit_clamp_various() {
    assert_eq!(
        zai_rs::client::transport::limits::Limit::clamp(0, 100).bytes,
        0
    );
    assert_eq!(
        zai_rs::client::transport::limits::Limit::clamp(50, 100).bytes,
        50
    );
    assert_eq!(
        zai_rs::client::transport::limits::Limit::clamp(100, 100).bytes,
        100
    );
    assert_eq!(
        zai_rs::client::transport::limits::Limit::clamp(200, 100).bytes,
        100
    );
}

// --- transport/redaction deeper ---
#[test]
fn redaction_various_inputs() {
    use zai_rs::client::transport::redaction::*;
    assert_eq!(sanitize_request_id("valid_id_123"), "valid_id_123");
    assert_eq!(sanitize_request_id("a\x00b\x01c"), "abc");
    assert!(has_usable_id(&sanitize_request_id("normal_id")));
    assert!(!has_usable_id(&sanitize_request_id("\x01\x02")));
    // Truncation
    let long = "x".repeat(200);
    let s = sanitize_request_id(&long);
    assert!(s.len() <= 128);
}

// --- transport/decode extract_error_envelope ---
#[test]
fn extract_envelope_various() {
    use zai_rs::client::transport::decode::extract_error_envelope;
    let e = extract_error_envelope(r#"{"code":1302,"message":"rate limited"}"#);
    assert!(e.is_some());
    let e = e.unwrap();
    assert!(e.message.contains("rate limited"));

    let e = extract_error_envelope(r#"{"error":{"code":500,"message":"x"}}"#);
    assert!(e.is_some());

    let e = extract_error_envelope(r#"{"code":200,"message":"ok"}"#);
    assert!(e.is_none());

    let e = extract_error_envelope(r#"{"choices":[]}"#);
    assert!(e.is_none());
}

// --- transport/redirect deeper ---
#[test]
fn redirect_no_follow_on_200() {
    use zai_rs::client::transport::redirect::follow;
    use zai_rs::client::transport::retry::RetrySafety;
    let cur = url::Url::parse("https://open.bigmodel.cn/a").unwrap();
    assert!(
        follow(&cur, 200, "/b", RetrySafety::Idempotent, "GET", 0)
            .unwrap()
            .is_none()
    );
    assert!(
        follow(&cur, 404, "/b", RetrySafety::Idempotent, "GET", 0)
            .unwrap()
            .is_none()
    );
}

#[test]
fn redirect_userinfo_fragment_rejected() {
    use zai_rs::client::transport::redirect::follow;
    use zai_rs::client::transport::retry::RetrySafety;
    let cur = url::Url::parse("https://open.bigmodel.cn/a").unwrap();
    assert!(
        follow(
            &cur,
            302,
            "https://u:p@open.bigmodel.cn/b",
            RetrySafety::Idempotent,
            "GET",
            0
        )
        .is_err()
    );
    assert!(
        follow(
            &cur,
            302,
            "https://open.bigmodel.cn/b#frag",
            RetrySafety::Idempotent,
            "GET",
            0
        )
        .is_err()
    );
}

#[test]
fn redirect_hop_limit() {
    use zai_rs::client::transport::redirect::{MAX_REDIRECTS, follow};
    use zai_rs::client::transport::retry::RetrySafety;
    let cur = url::Url::parse("https://open.bigmodel.cn/a").unwrap();
    for h in 0..MAX_REDIRECTS {
        assert!(follow(&cur, 302, "/b", RetrySafety::Idempotent, "GET", h).is_ok());
    }
    assert!(
        follow(
            &cur,
            302,
            "/b",
            RetrySafety::Idempotent,
            "GET",
            MAX_REDIRECTS
        )
        .is_err()
    );
}

// --- transport/multipart deeper ---
#[test]
fn multipart_factory_fields_and_build() {
    use zai_rs::client::transport::multipart::*;
    let dir = tempfile::tempdir().unwrap();
    let p1 = dir.path().join("a.txt");
    std::fs::write(&p1, b"hello").unwrap();
    let part = FilePart::from_path(&p1).unwrap();
    assert_eq!(part.filename, "a.txt");
    assert!(part.content_type == "text/plain" || part.content_type == "application/octet-stream");

    let factory = MultipartBodyFactory::new()
        .file(part)
        .unwrap()
        .field("key", "value")
        .unwrap();
    let form = factory.build().unwrap();
    let _ = form;
}

#[test]
fn multipart_content_type_guessing() {
    use zai_rs::client::transport::multipart::*;
    let dir = tempfile::tempdir().unwrap();
    for (ext, mime) in [
        ("png", "image/png"),
        ("jpg", "image/jpeg"),
        ("jpeg", "image/jpeg"),
        ("wav", "audio/wav"),
        ("mp3", "audio/mpeg"),
        ("pdf", "application/pdf"),
        ("mp4", "video/mp4"),
        ("xyz", "application/octet-stream"),
    ] {
        let p = dir.path().join(format!("test.{ext}"));
        std::fs::write(&p, b"x").unwrap();
        let part = FilePart::from_path(&p).unwrap();
        assert_eq!(part.content_type, mime, "extension .{ext}");
    }
}

#[cfg(feature = "realtime")]
mod realtime_cov {
    use zai_rs::realtime::events::ClientEvent;
    use zai_rs::realtime::protocol::*;

    #[test]
    fn client_event_session_update_serialize() {
        let event = ClientEvent::SessionUpdate {
            event_id: Some("e1".into()),
            session: SessionConfig::default(),
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "session.update");
    }

    #[test]
    fn client_event_input_audio_append() {
        let event = ClientEvent::InputAudioBufferAppend {
            audio: "base64".into(),
            client_timestamp: None,
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "input_audio_buffer.append");
    }

    #[test]
    fn session_config_default() {
        let cfg = SessionConfig::default();
        assert!(cfg.instructions.is_none());
    }

    #[test]
    fn session_config_with_instructions() {
        let cfg = SessionConfig {
            instructions: Some("be helpful".into()),
            ..SessionConfig::default()
        };
        let json = serde_json::to_value(&cfg).unwrap();
        assert!(json["instructions"].is_string());
    }

    #[test]
    fn turn_detection_default() {
        let td = TurnDetection {
            type_: TurnDetectionType::ServerVad,
        };
        let json = serde_json::to_value(&td).unwrap();
        assert_eq!(json["type"], "server_vad");
    }

    #[test]
    fn beta_fields_default() {
        let bf = BetaFields::default();
        assert!(bf.chat_mode.is_none());
    }

    #[test]
    fn chat_mode_variants() {
        assert_eq!(
            serde_json::to_string(&ChatMode::Audio).unwrap(),
            r#""audio""#
        );
        assert_eq!(
            serde_json::to_string(&ChatMode::VideoPassive).unwrap(),
            r#""video_passive""#
        );
    }

    #[test]
    fn realtime_tool_new() {
        let tool =
            RealtimeTool::function("calc", "calculator", serde_json::json!({"type":"object"}));
        let _ = tool;
    }
}
