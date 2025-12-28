
use zai_rs::real_time::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Test ClientEvent serialization
    println!("=== Client Events ===");

    // Session update event
    let session = Session {
        model: Some("glm-realtime".to_string()),
        modalities: Some(vec!["text".to_string(), "audio".to_string()]),
        voice: Some("tongtong".to_string()),
        ..Default::default()
    };

    let mut session_update_event = SessionUpdateEvent {
        event_id: Some("session-123".to_string()),
        client_timestamp: Some(1625097600000),
        ..Default::default()
    };
    session_update_event.set_session(session);

    let client_event = ClientEvent::SessionUpdate(session_update_event);
    let json = serde_json::to_string_pretty(&client_event)?;
    println!("Session Update Event:");
    println!("{}", json);
    println!();

    // Input audio buffer append event
    let audio_append_event = InputAudioBufferAppendEvent {
        event_id: Some("audio-123".to_string()),
        client_timestamp: Some(1625097600000),
        audio: "UklGRiQZAABXQVZFZm10IBAAAAABAAEAgD4AAAB9AAACABAAZGF0YQAZAAAR9Hrx...".to_string(),
    };

    let client_event = ClientEvent::InputAudioBufferAppend(audio_append_event);
    let json = serde_json::to_string_pretty(&client_event)?;
    println!("Input Audio Buffer Append Event:");
    println!("{}", json);
    println!();

    // Test ServerEvent serialization
    println!("=== Server Events ===");

    // Session created event
    let session_created_event = SessionCreatedEvent {
        event_id: Some("event-123".to_string()),
        client_timestamp: Some(1625097600000),
        session: Session::default(),
    };

    let server_event = ServerEvent::SessionCreated(session_created_event);
    let json = serde_json::to_string_pretty(&server_event)?;
    println!("Session Created Event:");
    println!("{}", json);
    println!();

    // Response created event
    let response_created_event = ResponseCreatedEvent {
        event_id: Some("event-456".to_string()),
        client_timestamp: Some(1625097601000),
        response: RealtimeResponse::default(),
    };

    let server_event = ServerEvent::ResponseCreated(response_created_event);
    let json = serde_json::to_string_pretty(&server_event)?;
    println!("Response Created Event:");
    println!("{}", json);

    Ok(())
}
