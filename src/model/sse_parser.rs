//! Shared SSE (Server-Sent Events) line parsing utilities.
//!
//! Extracts the common logic of buffering raw byte chunks, splitting on `\n`,
//! trimming `\r\n`, and yielding `data: ` prefixed payload lines.

/// Incremental SSE event parser.
///
/// Unlike [`extract_sse_data_lines`], this parser follows event boundaries and
/// joins multiple `data:` lines in the same event with `\n`.
#[derive(Debug, Default)]
pub struct SseEventParser {
    buf: Vec<u8>,
    event_data: Vec<Vec<u8>>,
}

impl SseEventParser {
    /// Create a new empty SSE event parser.
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a transport byte chunk and return completed SSE event payloads.
    pub fn push(&mut self, new_bytes: &[u8]) -> Vec<Vec<u8>> {
        self.buf.extend_from_slice(new_bytes);
        let mut events = Vec::with_capacity(4);

        // Scan forward without a per-line `drain` (which memmoves the entire
        // tail on every line → O(n^2) per chunk, and this runs for every
        // streaming token). Track consumed bytes and drop them once at the end.
        let mut consumed = 0;
        while let Some(rel) = self.buf[consumed..].iter().position(|&b| b == b'\n') {
            let newline = consumed + rel;
            // Line body is `[consumed, end)`; strip a single trailing CR.
            let end = if newline > consumed && self.buf[newline - 1] == b'\r' {
                newline - 1
            } else {
                newline
            };
            let line = &self.buf[consumed..end];
            consumed = newline + 1;

            if line.is_empty() {
                // Common case (exactly one `data:` line per event — the norm for
                // chat token streams): move the single buffer straight out and
                // skip the extra allocation+copy that `join_event_data` would
                // do. Multi-line events still join (and keep buffer reuse).
                match self.event_data.len() {
                    0 => {},
                    1 => {
                        events.push(self.event_data.swap_remove(0));
                    },
                    _ => {
                        events.push(join_event_data(&self.event_data));
                        self.event_data.clear();
                    },
                }
                continue;
            }

            if line.starts_with(b":") {
                continue;
            }

            if let Some(rest) = line.strip_prefix(b"data:") {
                self.event_data.push(trim_one_leading_space(rest).to_vec());
            }
        }

        self.buf.drain(..consumed);
        events
    }

    /// Flush any event buffered by `data:` lines that never saw a terminating
    /// blank line.
    ///
    /// Per the SSE spec an event is dispatched on a blank line. If the transport
    /// closes after a final `data: {...}\n` with **no** following blank line
    /// (a reverse proxy stripping trailing whitespace, a truncated TLS frame at
    /// connection close, a non-conformant emitter), [`SseEventParser::push`]
    /// leaves that event buffered in `event_data` and it would otherwise be
    /// silently dropped — including the last content/usage chunk, or even the
    /// `[DONE]` marker if its trailing blank line was lost.
    ///
    /// Call this once the byte stream has ended to emit any such trailing event.
    /// Returns an empty `Vec` when nothing is buffered. Any incomplete line
    /// still in `buf` (a `data:` line with no trailing newline) is intentionally
    /// NOT emitted — it is not a complete SSE line.
    pub fn finish(&mut self) -> Vec<Vec<u8>> {
        match self.event_data.len() {
            0 => Vec::new(),
            1 => vec![self.event_data.swap_remove(0)],
            _ => {
                let event = join_event_data(&self.event_data);
                self.event_data.clear();
                vec![event]
            },
        }
    }
}

fn trim_one_leading_space(bytes: &[u8]) -> &[u8] {
    bytes.strip_prefix(b" ").unwrap_or(bytes)
}

fn join_event_data(lines: &[Vec<u8>]) -> Vec<u8> {
    let total: usize = lines.iter().map(std::vec::Vec::len).sum::<usize>() + lines.len();
    let mut event = Vec::with_capacity(total);
    for (idx, line) in lines.iter().enumerate() {
        if idx > 0 {
            event.push(b'\n');
        }
        event.extend_from_slice(line);
    }
    event
}

/// Process a new chunk of bytes, extract completed SSE data lines.
///
/// Appends `new_bytes` to `buf`, then extracts all complete lines (delimited
/// by `\n`). For each line, it:
/// - Strips trailing `\r` and `\n`
/// - Skips empty lines
/// - Strips the `"data: "` prefix and yields the remaining bytes
///
/// Returns owned byte vectors containing each data payload.
/// Lines that are not prefixed with `"data: "` are silently skipped.
///
/// If a `data: [DONE]` line is encountered, it is yielded as a
/// `[b"[DONE]"]` entry so the caller can detect stream termination.
pub fn extract_sse_data_lines(buf: &mut Vec<u8>, new_bytes: &[u8]) -> Vec<Vec<u8>> {
    buf.extend_from_slice(new_bytes);
    let mut results = Vec::new();

    let Some(last_newline) = buf.iter().rposition(|&b| b == b'\n') else {
        return results;
    };

    let completed = &buf[..=last_newline];
    for line_with_nl in completed.split_inclusive(|&b| b == b'\n') {
        let mut line = line_with_nl;
        if let Some(line_without_nl) = line.strip_suffix(b"\n") {
            line = line_without_nl;
        }
        if let Some(line_without_cr) = line.strip_suffix(b"\r") {
            line = line_without_cr;
        }
        if line.is_empty() {
            continue;
        }
        const PREFIX: &[u8] = b"data: ";
        if let Some(rest) = line.strip_prefix(PREFIX) {
            results.push(rest.to_vec());
        }
    }

    buf.drain(..=last_newline);

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_complete_line() {
        let mut buf = Vec::new();
        let lines = extract_sse_data_lines(&mut buf, b"data: hello\n");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], b"hello");
    }

    #[test]
    fn test_partial_then_complete() {
        let mut buf = Vec::new();
        let lines1 = extract_sse_data_lines(&mut buf, b"data: hel");
        assert!(lines1.is_empty());

        let lines2 = extract_sse_data_lines(&mut buf, b"lo\n");
        assert_eq!(lines2.len(), 1);
        assert_eq!(lines2[0], b"hello");
    }

    #[test]
    fn test_crlf_line_endings() {
        let mut buf = Vec::new();
        let lines = extract_sse_data_lines(&mut buf, b"data: world\r\n");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], b"world");
    }

    #[test]
    fn test_multiple_events_in_one_chunk() {
        let mut buf = Vec::new();
        let lines = extract_sse_data_lines(&mut buf, b"data: first\n\ndata: second\n");
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], b"first");
        assert_eq!(lines[1], b"second");
    }

    #[test]
    fn test_done_marker() {
        let mut buf = Vec::new();
        let lines = extract_sse_data_lines(&mut buf, b"data: [DONE]\n");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], b"[DONE]");
    }

    #[test]
    fn test_non_data_lines_skipped() {
        let mut buf = Vec::new();
        let lines = extract_sse_data_lines(&mut buf, b": comment\nid: 123\ndata: payload\n");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], b"payload");
    }

    #[test]
    fn test_empty_lines_ignored() {
        let mut buf = Vec::new();
        let lines = extract_sse_data_lines(&mut buf, b"\n\n\ndata: hello\n\n");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], b"hello");
    }

    #[test]
    fn event_parser_yields_complete_events() {
        let mut parser = SseEventParser::new();
        assert!(parser.push(b"data: hel").is_empty());
        assert_eq!(parser.push(b"lo\r\n\r\n"), vec![b"hello".to_vec()]);
    }

    #[test]
    fn event_parser_joins_multi_data_lines() {
        let mut parser = SseEventParser::new();
        let events = parser.push(b"data: {\"a\":\ndata: 1}\n\n");
        assert_eq!(events, vec![b"{\"a\":\n1}".to_vec()]);
    }

    #[test]
    fn event_parser_ignores_comments_and_non_data_fields() {
        let mut parser = SseEventParser::new();
        let events = parser.push(b": keepalive\nid: 1\ndata: payload\n\n");
        assert_eq!(events, vec![b"payload".to_vec()]);
    }

    #[test]
    fn event_parser_done_marker_split_across_chunks() {
        // Regression guard for the scan-in-place rewrite: a `[DONE]` payload
        // split across two transport chunks must still reassemble into one
        // `[DONE]` event.
        let mut parser = SseEventParser::new();
        assert!(parser.push(b"data: [DO").is_empty());
        assert_eq!(parser.push(b"NE]\n\n"), vec![b"[DONE]".to_vec()]);
    }

    #[test]
    fn event_parser_lone_cr_then_lf() {
        // A CR ending one chunk followed by an LF starting the next must not
        // leave a stray CR in the payload.
        let mut parser = SseEventParser::new();
        assert!(parser.push(b"data: hi\r").is_empty());
        assert_eq!(parser.push(b"\n\n"), vec![b"hi".to_vec()]);
    }

    #[test]
    fn extract_sse_handles_crlf_split_across_chunks() {
        let mut buf = Vec::new();
        assert!(extract_sse_data_lines(&mut buf, b"data: hello\r").is_empty());
        let lines = extract_sse_data_lines(&mut buf, b"\n");
        assert_eq!(lines, vec![b"hello".to_vec()]);
    }

    #[test]
    fn finish_flushes_trailing_event_without_blank_line() {
        // Regression: a final `data:` line whose terminating blank line was
        // lost (truncated frame, proxy stripping trailing whitespace, a
        // non-conformant emitter) must still be emitted via finish() rather
        // than silently dropped — including the last content/[DONE] chunk.
        let mut parser = SseEventParser::new();
        assert!(parser.push(b"data: hello\n").is_empty()); // no blank line -> buffered
        assert_eq!(parser.finish(), vec![b"hello".to_vec()]);
        // finish() is idempotent.
        assert!(parser.finish().is_empty());
    }

    #[test]
    fn finish_flushes_multi_data_trailing_event_joined() {
        let mut parser = SseEventParser::new();
        assert!(parser.push(b"data: {\"a\":\ndata: 1}\n").is_empty());
        assert_eq!(parser.finish(), vec![b"{\"a\":\n1}".to_vec()]);
    }

    #[test]
    fn finish_noop_when_event_already_dispatched() {
        let mut parser = SseEventParser::new();
        assert_eq!(parser.push(b"data: hello\n\n"), vec![b"hello".to_vec()]);
        assert!(parser.finish().is_empty());
    }

    #[test]
    fn finish_emits_trailing_done_marker_without_blank_line() {
        // The terminal [DONE] can also lose its trailing blank line; it must
        // still surface so consumers can stop.
        let mut parser = SseEventParser::new();
        assert!(parser.push(b"data: [DONE]\n").is_empty());
        assert_eq!(parser.finish(), vec![b"[DONE]".to_vec()]);
    }
}
