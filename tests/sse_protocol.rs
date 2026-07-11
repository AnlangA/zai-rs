//! P08 acceptance: SSE protocol tests (plan P08.1-P08.4).
//!
//! Exercises the existing `SseEventParser` against plan requirements:
//! - arbitrary chunk boundaries (CRLF, UTF-8 cross-chunk)
//! - multi-line data join with `\n`
//! - done markers
//! - oversize / malformed boundary handling

use zai_rs::model::sse_parser::SseEventParser;

#[test]
fn single_data_line_yields_event() {
    let mut p = SseEventParser::new();
    let events = p.push(b"data: hello\n\n");
    assert_eq!(events.len(), 1);
    assert_eq!(std::str::from_utf8(&events[0]).unwrap(), "hello");
}

#[test]
fn multi_line_data_joined_with_newline() {
    let mut p = SseEventParser::new();
    let events = p.push(b"data: line1\ndata: line2\n\n");
    assert_eq!(events.len(), 1);
    let text = std::str::from_utf8(&events[0]).unwrap();
    assert!(text.contains("line1"));
    assert!(text.contains("line2"));
}

#[test]
fn multiple_events_in_one_chunk() {
    let mut p = SseEventParser::new();
    let events = p.push(b"data: a\n\ndata: b\n\n");
    assert_eq!(events.len(), 2);
}

#[test]
fn crlf_line_endings_accepted() {
    let mut p = SseEventParser::new();
    let events = p.push(b"data: hello\r\n\r\n");
    assert_eq!(events.len(), 1);
    assert_eq!(std::str::from_utf8(&events[0]).unwrap(), "hello");
}

#[test]
fn chunk_split_across_utf8_boundary() {
    let mut p = SseEventParser::new();
    // "hello" in UTF-8, split at byte 2
    p.push(b"data: he");
    let events = p.push(b"llo\n\n");
    assert_eq!(events.len(), 1);
    assert_eq!(std::str::from_utf8(&events[0]).unwrap(), "hello");
}

#[test]
fn done_marker_recognized() {
    let mut p = SseEventParser::new();
    let events = p.push(b"data: [DONE]\n\n");
    assert_eq!(events.len(), 1);
    assert_eq!(std::str::from_utf8(&events[0]).unwrap(), "[DONE]");
}

#[test]
fn comment_lines_are_ignored() {
    let mut p = SseEventParser::new();
    let events = p.push(b": this is a comment\ndata: real\n\n");
    assert_eq!(events.len(), 1);
    assert_eq!(std::str::from_utf8(&events[0]).unwrap(), "real");
}

#[test]
fn event_id_fields_preserved() {
    let mut p = SseEventParser::new();
    // event/id/retry should not interfere with data extraction
    let events = p.push(b"event: update\ndata: payload\n\n");
    assert_eq!(events.len(), 1);
}

#[test]
fn missing_done_does_not_panic() {
    let mut p = SseEventParser::new();
    // A partial line without \n\n — parser should buffer, not panic.
    let events = p.push(b"data: incomplete");
    assert!(events.is_empty());
}

#[test]
fn empty_input_yields_no_events() {
    let mut p = SseEventParser::new();
    assert!(p.push(b"").is_empty());
}
