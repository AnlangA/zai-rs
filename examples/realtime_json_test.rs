use serde_json;
use zai_rs::real_time::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Test ClientEvent serialization
    println!("=== Client Events ===");

    // Session update event
    let mut session = Session::default();
    session.model = Some("glm-realtime".to_string());
    session.modalities = Some(vec!["text".to_string(), "audio".to_string()]);
    session.voice = Some("tongtong".to_string());

    let mut session_update_event = SessionUpdateEvent::default();
    session_update_event.event_id = Some("session-123".to_string());
    session_update_event.client_timestamp = Some(1625097600000);
    session_update_event.set_session(session);

    let client_event = ClientEvent::SessionUpdate(session_update_event);
    let json = serde_json::to_string_pretty(&client_event)?;
    println!("Session Update Event:");
    println!("{}", json);
    println!();

    // Input audio buffer append event
    let mut audio_append_event = InputAudioBufferAppendEvent::default();
    audio_append_event.event_id = Some("audio-123".to_string());
    audio_append_event.client_timestamp = Some(1625097600000);
    audio_append_event.audio =
        "UklGRiQZAABXQVZFZm10IBAAAAABAAEAgD4AAAB9AAACABAAZGF0YQAZAAAR9Hrx...".to_string();

    let client_event = ClientEvent::InputAudioBufferAppend(audio_append_event);
    let json = serde_json::to_string_pretty(&client_event)?;
    println!("Input Audio Buffer Append Event:");
    println!("{}", json);
    println!();

    // Test ServerEvent serialization
    println!("=== Server Events ===");

    // Session created event
    let mut session_created_event = SessionCreatedEvent::default();
    session_created_event.event_id = Some("event-123".to_string());
    session_created_event.client_timestamp = Some(1625097600000);
    session_created_event.session = Session::default();

    let server_event = ServerEvent::SessionCreated(session_created_event);
    let json = serde_json::to_string_pretty(&server_event)?;
    println!("Session Created Event:");
    println!("{}", json);
    println!();

    // Response created event
    let mut response_created_event = ResponseCreatedEvent::default();
    response_created_event.event_id = Some("event-456".to_string());
    response_created_event.client_timestamp = Some(1625097601000);
    response_created_event.response = RealtimeResponse::default();

    let server_event = ServerEvent::ResponseCreated(response_created_event);
    let json = serde_json::to_string_pretty(&server_event)?;
    println!("Response Created Event:");
    println!("{}", json);

    Ok(())
}
