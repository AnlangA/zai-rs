//! Shared SSE (Server-Sent Events) line parsing utilities.
//!
//! Extracts the common logic of buffering fragmented bytes, recognizing LF,
//! CRLF, and lone-CR line endings, ignoring an initial UTF-8 BOM, and yielding
//! joined `data:` field payloads.

use std::pin::Pin;

use futures_util::{Stream, StreamExt};

use crate::{ZaiError, ZaiResult, client::error::codes};

pub(crate) type DecodedSseStream<T> = Pin<Box<dyn Stream<Item = ZaiResult<T>> + Send + 'static>>;

/// Incremental SSE event parser.
///
/// Unlike [`extract_sse_data_lines`], this parser follows event boundaries and
/// joins multiple `data:` lines in the same event with `\n`.
#[derive(Debug, Default)]
pub struct SseEventParser {
    buf: Vec<u8>,
    scan_from: usize,
    // First byte that has not already been inspected for a line terminator.
    // This is distinct from `scan_from`, which remains the start of the
    // current line so its full payload can be sliced once a terminator arrives.
    search_from: usize,
    // Joined event payload. Keeping one contiguous buffer avoids one owned
    // allocation per `data:` line and the full-event copy at dispatch.
    event_data: Vec<u8>,
    event_data_lines: usize,
    bom_checked: bool,
}

impl SseEventParser {
    /// Create a new empty SSE event parser.
    pub fn new() -> Self {
        Self::default()
    }

    /// Bytes retained for the current incomplete line/event.
    pub(crate) fn buffered_len(&self) -> usize {
        self.buf
            .len()
            .saturating_sub(self.scan_from)
            .saturating_add(self.event_data.len())
    }

    /// Push a transport byte chunk and return completed SSE event payloads.
    ///
    /// This compatibility method enforces the same 32 MiB / 4096-line limits as
    /// production streams. If a limit is exceeded it resets the parser and
    /// returns no events because its historical return type cannot carry an
    /// error. New code should use [`Self::try_push`] to observe that failure.
    pub fn push(&mut self, new_bytes: &[u8]) -> Vec<Vec<u8>> {
        self.try_push(new_bytes).unwrap_or_default()
    }

    /// Push a transport byte chunk with bounded memory and explicit errors.
    ///
    /// Input is processed in small slices so one unusually large transport
    /// chunk cannot be copied wholesale into parser-owned memory. Each event is
    /// limited to 32 MiB and 4096 `data:` lines; on failure all retained bytes
    /// are released and the parser can be reused.
    pub fn try_push(&mut self, new_bytes: &[u8]) -> ZaiResult<Vec<Vec<u8>>> {
        let result = self.try_push_inner(new_bytes);
        if result.is_err() {
            self.reset();
        }
        result
    }

    fn try_push_inner(&mut self, new_bytes: &[u8]) -> ZaiResult<Vec<Vec<u8>>> {
        // Most transport chunks end in the middle of an event. Avoid eagerly
        // allocating return storage on that overwhelmingly common empty path;
        // `Vec` will allocate only when this call actually completes an event.
        let mut events = Vec::new();
        for bytes in new_bytes.chunks(crate::client::transport::limits::SSE_PARSE_SLICE_BYTES) {
            self.feed(bytes);
            while let Some(event) = self.next_bounded()? {
                events.push(event);
            }
            if self.buffered_len() > crate::client::transport::limits::SSE_PARSER_RETAINED_MAX {
                return Err(event_too_large());
            }
        }
        Ok(events)
    }

    /// Append bytes without eagerly collecting all completed events.
    pub(crate) fn feed(&mut self, new_bytes: &[u8]) {
        self.buf.extend_from_slice(new_bytes);
    }

    /// Return at most one event while enforcing the production stream limits.
    pub(crate) fn next_bounded(&mut self) -> ZaiResult<Option<Vec<u8>>> {
        self.next_event_with_limits(
            crate::client::transport::limits::SSE_EVENT_BYTES_MAX,
            crate::client::transport::limits::SSE_EVENT_DATA_LINES_MAX,
            false,
        )
    }

    /// Finish the input and return at most one remaining event.
    pub(crate) fn finish_next_bounded(&mut self) -> ZaiResult<Option<Vec<u8>>> {
        if let Some(event) = self.next_event_with_limits(
            crate::client::transport::limits::SSE_EVENT_BYTES_MAX,
            crate::client::transport::limits::SSE_EVENT_DATA_LINES_MAX,
            true,
        )? {
            return Ok(Some(event));
        }
        if self.event_data_lines == 0 {
            self.buf.clear();
            self.scan_from = 0;
            self.search_from = 0;
            return Ok(None);
        }
        let event = self.take_event();
        self.buf.clear();
        self.scan_from = 0;
        self.search_from = 0;
        Ok(Some(event))
    }

    fn next_event_with_limits(
        &mut self,
        max_event_bytes: usize,
        max_data_lines: usize,
        eof: bool,
    ) -> ZaiResult<Option<Vec<u8>>> {
        if !self.ensure_bom_checked(eof) {
            return Ok(None);
        }

        loop {
            let Some((line_start, line_end, next_line)) = self.next_line(eof) else {
                self.maybe_compact();
                return Ok(None);
            };

            let is_empty = line_start == line_end;
            let is_comment = !is_empty && self.buf[line_start] == b':';
            let data = (!is_empty && !is_comment)
                .then(|| self.buf[line_start..line_end].strip_prefix(b"data:"))
                .flatten()
                .map(trim_one_leading_space);

            self.scan_from = next_line;

            if is_empty {
                if self.event_data_lines == 0 {
                    self.maybe_compact();
                    continue;
                }
                let event = self.take_event();
                self.maybe_compact();
                return Ok(Some(event));
            }

            if let Some(data) = data {
                if self.event_data_lines >= max_data_lines {
                    return Err(event_has_too_many_lines(max_data_lines));
                }
                let separator = usize::from(self.event_data_lines != 0);
                let next_bytes = self
                    .event_data
                    .len()
                    .checked_add(separator)
                    .and_then(|bytes| bytes.checked_add(data.len()))
                    .ok_or_else(event_too_large)?;
                if next_bytes > max_event_bytes {
                    return Err(event_too_large());
                }
                if separator != 0 {
                    self.event_data.push(b'\n');
                }
                self.event_data.extend_from_slice(data);
                self.event_data_lines += 1;
            }

            self.maybe_compact();
        }
    }

    fn ensure_bom_checked(&mut self, eof: bool) -> bool {
        if self.bom_checked {
            return true;
        }

        const UTF8_BOM: &[u8; 3] = b"\xef\xbb\xbf";
        let pending = &self.buf[self.scan_from..];
        if !eof && pending.len() < UTF8_BOM.len() && UTF8_BOM.starts_with(pending) {
            return false;
        }
        if pending.starts_with(UTF8_BOM) {
            self.scan_from += UTF8_BOM.len();
            self.search_from = self.search_from.max(self.scan_from);
        }
        self.bom_checked = true;
        self.maybe_compact();
        true
    }

    fn next_line(&mut self, eof: bool) -> Option<(usize, usize, usize)> {
        let bytes = &self.buf;
        let mut cursor = self.search_from.max(self.scan_from);
        while cursor < bytes.len() {
            match bytes[cursor] {
                b'\n' => {
                    let next_line = cursor + 1;
                    self.search_from = next_line;
                    return Some((self.scan_from, cursor, next_line));
                },
                b'\r' if cursor + 1 < bytes.len() => {
                    let terminator_len = usize::from(bytes[cursor + 1] == b'\n') + 1;
                    let next_line = cursor + terminator_len;
                    self.search_from = next_line;
                    return Some((self.scan_from, cursor, next_line));
                },
                b'\r' if eof => {
                    let next_line = cursor + 1;
                    self.search_from = next_line;
                    return Some((self.scan_from, cursor, next_line));
                },
                // A CR at the end of a transport chunk is ambiguous until the
                // next byte arrives: it may be a lone terminator or the first
                // half of CRLF. Revisit only this byte on the next feed.
                b'\r' => {
                    self.search_from = cursor;
                    return None;
                },
                _ => cursor += 1,
            }
        }
        self.search_from = bytes.len();
        None
    }

    fn take_event(&mut self) -> Vec<u8> {
        self.event_data_lines = 0;
        std::mem::take(&mut self.event_data)
    }

    fn maybe_compact(&mut self) {
        const COMPACT_THRESHOLD: usize = 64 * 1024;
        // Normal transport parsing feeds at most one parse slice at a time.
        // A syntactically valid multi-megabyte comment or unknown SSE field
        // can nevertheless grow `buf` close to the full event limit before
        // its newline arrives. Once fully consumed, do not let that attacker-
        // sized scratch allocation remain pinned for the rest of a long-lived
        // stream.
        const RETAINED_SCRATCH_MAX: usize =
            crate::client::transport::limits::SSE_PARSE_SLICE_BYTES * 2;
        if self.scan_from == 0 {
            return;
        }
        if self.scan_from == self.buf.len() {
            if self.buf.capacity() > RETAINED_SCRATCH_MAX {
                self.buf = Vec::new();
            } else {
                self.buf.clear();
            }
            self.scan_from = 0;
            self.search_from = 0;
        } else if self.scan_from >= COMPACT_THRESHOLD && self.scan_from >= self.buf.len() / 2 {
            let drained = self.scan_from;
            if self.buf.capacity() > RETAINED_SCRATCH_MAX {
                // Preserve an incomplete following line without preserving the
                // oversized allocation that preceded it. `to_vec` performs the
                // same unavoidable byte move as `drain`, but right-sizes the
                // replacement buffer.
                self.buf = self.buf[drained..].to_vec();
            } else {
                self.buf.drain(..drained);
            }
            self.scan_from = 0;
            self.search_from = self.search_from.saturating_sub(drained);
        }
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
        self.try_finish().unwrap_or_default()
    }

    /// Finish the input with the production event limits and explicit errors.
    ///
    /// Like [`Self::try_push`], a failure releases all parser-owned buffers.
    pub fn try_finish(&mut self) -> ZaiResult<Vec<Vec<u8>>> {
        let result = self.try_finish_inner();
        if result.is_err() {
            self.reset();
        }
        result
    }

    fn try_finish_inner(&mut self) -> ZaiResult<Vec<Vec<u8>>> {
        let mut events = Vec::new();
        while let Some(event) = self.next_event_with_limits(
            crate::client::transport::limits::SSE_EVENT_BYTES_MAX,
            crate::client::transport::limits::SSE_EVENT_DATA_LINES_MAX,
            true,
        )? {
            events.push(event);
        }
        if self.event_data_lines != 0 {
            events.push(self.take_event());
        }
        self.reset();
        Ok(events)
    }

    fn reset(&mut self) {
        // Assignment, rather than `clear`, releases a potentially attacker-
        // sized allocation immediately after a rejected public parse.
        self.buf = Vec::new();
        self.scan_from = 0;
        self.search_from = 0;
        self.event_data = Vec::new();
        self.event_data_lines = 0;
        self.bom_checked = false;
    }
}

struct RequiredDoneState {
    raw: Option<crate::client::transport::SseByteStream>,
    parser: SseEventParser,
    current_chunk: Option<bytes::Bytes>,
    chunk_offset: usize,
    input_finished: bool,
    terminated: bool,
}

/// Decode a typed SSE stream whose successful completion requires `[DONE]`.
///
/// Transport failures, oversized events, malformed items, and in-band business
/// errors are each yielded once and then terminate the stream. EOF without the
/// terminal marker is an error rather than normal completion.
pub(crate) fn decode_required_done_stream<T, F>(
    raw: crate::client::transport::SseByteStream,
    decode: F,
) -> DecodedSseStream<T>
where
    T: Send + 'static,
    F: Fn(&[u8]) -> ZaiResult<T> + Send + 'static,
{
    let payloads = required_done_payloads(raw);
    let stream = futures_util::stream::unfold(
        (Some(payloads), decode, false),
        |(mut payloads, decode, terminated)| async move {
            if terminated {
                return None;
            }
            let item = payloads.as_mut()?.next().await?;
            let item = item.and_then(|payload| decode(&payload));
            let terminated = item.is_err();
            if terminated {
                // A terminal decode error must release the authenticated raw
                // response immediately, even if the caller keeps this stream
                // without polling it again.
                payloads = None;
            }
            Some((item, (payloads, decode, terminated)))
        },
    );
    Box::pin(stream)
}

fn required_done_payloads(
    raw: crate::client::transport::SseByteStream,
) -> Pin<Box<dyn Stream<Item = ZaiResult<Vec<u8>>> + Send + 'static>> {
    let state = RequiredDoneState {
        raw: Some(raw),
        parser: SseEventParser::new(),
        current_chunk: None,
        chunk_offset: 0,
        input_finished: false,
        terminated: false,
    };
    let stream = futures_util::stream::unfold(state, |mut state| async move {
        loop {
            if state.terminated {
                return None;
            }

            let next_payload = if state.input_finished {
                state.parser.finish_next_bounded()
            } else {
                state.parser.next_bounded()
            };
            let payload = match next_payload {
                Ok(payload) => payload,
                Err(error) => {
                    state.terminate();
                    return Some((Err(error), state));
                },
            };

            if let Some(payload) = payload {
                if payload == b"[DONE]" {
                    return None;
                }
                if let Ok(payload_text) = std::str::from_utf8(&payload) {
                    match crate::client::transport::decode::probe_error_envelope(payload_text) {
                        crate::client::transport::decode::ProbeOutcome::Error(error) => {
                            state.terminate();
                            return Some((Err(business_error(error)), state));
                        },
                        crate::client::transport::decode::ProbeOutcome::Ambiguous => {
                            state.terminate();
                            return Some((Err(ambiguous_business_error()), state));
                        },
                        crate::client::transport::decode::ProbeOutcome::Clean
                        | crate::client::transport::decode::ProbeOutcome::Malformed => {},
                    }
                }
                return Some((Ok(payload), state));
            }

            if state.input_finished {
                state.terminate();
                return Some((Err(ended_without_done()), state));
            }

            if state.parser.buffered_len()
                > crate::client::transport::limits::SSE_PARSER_RETAINED_MAX
            {
                state.terminate();
                return Some((Err(event_too_large()), state));
            }

            if let Some(chunk) = state.current_chunk.take() {
                let end = state
                    .chunk_offset
                    .saturating_add(crate::client::transport::limits::SSE_PARSE_SLICE_BYTES)
                    .min(chunk.len());
                state.parser.feed(&chunk[state.chunk_offset..end]);
                if end < chunk.len() {
                    state.current_chunk = Some(chunk);
                    state.chunk_offset = end;
                } else {
                    state.chunk_offset = 0;
                }
                continue;
            }

            let Some(raw) = state.raw.as_mut() else {
                state.terminate();
                return Some((Err(ended_without_done()), state));
            };
            match raw.next().await {
                Some(Ok(chunk)) if chunk.is_empty() => {},
                Some(Ok(chunk)) => state.current_chunk = Some(chunk),
                Some(Err(error)) => {
                    state.terminate();
                    return Some((Err(error), state));
                },
                None => state.input_finished = true,
            }
        }
    });
    Box::pin(stream)
}

impl RequiredDoneState {
    fn terminate(&mut self) {
        self.terminated = true;
        self.raw = None;
        self.current_chunk = None;
        self.parser.reset();
    }
}

fn event_too_large() -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_VALIDATION,
        message: format!(
            "SSE event exceeded limit ({} bytes)",
            crate::client::transport::limits::SSE_EVENT_BYTES_MAX
        ),
    }
}

fn event_has_too_many_lines(limit: usize) -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_VALIDATION,
        message: format!("SSE event exceeded data-line limit ({limit})"),
    }
}

fn ended_without_done() -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_IO,
        message: "SSE stream ended before the required [DONE] event".to_string(),
    }
}

fn business_error(error: crate::client::transport::decode::BusinessError) -> ZaiError {
    let code = error
        .code
        .as_ref()
        .and_then(crate::client::transport::parse_business_code)
        .unwrap_or_default();
    ZaiError::from_api_response(200, code, error.message)
}

fn ambiguous_business_error() -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_VALIDATION,
        message: "ambiguous JSON business-error envelope (duplicate reserved field)".to_string(),
    }
}

fn trim_one_leading_space(bytes: &[u8]) -> &[u8] {
    bytes.strip_prefix(b" ").unwrap_or(bytes)
}

/// Process a new chunk of bytes, extract completed SSE data lines.
///
/// Appends `new_bytes` to `buf`, then extracts all complete lines (delimited
/// by `\n`). For each line, it:
/// - Strips trailing `\r` and `\n`
/// - Skips empty lines
/// - Strips the `data:` prefix and its optional single leading space
///
/// Return owned byte vectors containing each `data:` payload.
/// Lines that are not `data:` fields are silently skipped.
///
/// If a `data: [DONE]` line is encountered, it is yielded as a
/// `[b"[DONE]"]` entry so the caller can detect stream termination.
///
/// `buf` must be the incomplete-line buffer preserved from the previous call
/// to this helper; after every call it contains no `\n`. This low-level
/// compatibility helper does not impose a size limit. For untrusted streams,
/// prefer [`SseEventParser::try_push`], which enforces bounded incremental
/// parsing and reports violations.
pub fn extract_sse_data_lines(buf: &mut Vec<u8>, new_bytes: &[u8]) -> Vec<Vec<u8>> {
    // A completed prior call leaves no newline in `buf`, so only the new chunk
    // can establish the last completed line. Searching just that chunk avoids
    // rescanning an ever-growing partial line for every tiny network fragment.
    let Some(last_newline_in_chunk) = new_bytes.iter().rposition(|&byte| byte == b'\n') else {
        buf.extend_from_slice(new_bytes);
        return Vec::new();
    };
    let old_len = buf.len();
    buf.extend_from_slice(new_bytes);
    let mut results = Vec::new();
    let last_newline = old_len + last_newline_in_chunk;

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
        if let Some(rest) = line.strip_prefix(b"data:") {
            results.push(trim_one_leading_space(rest).to_vec());
        }
    }

    buf.drain(..=last_newline);

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    #[test]
    fn test_single_complete_line() {
        let mut buf = Vec::new();
        let lines = extract_sse_data_lines(&mut buf, b"data: hello\n");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], b"hello");
    }

    #[test]
    fn test_data_field_without_optional_space() {
        let mut buf = Vec::new();
        let lines = extract_sse_data_lines(&mut buf, b"data:hello\n");
        assert_eq!(lines, vec![b"hello".to_vec()]);
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
    fn partial_line_only_scans_new_fragments_until_newline_arrives() {
        let mut buf = Vec::new();
        for byte in b"data: fragmented" {
            assert!(extract_sse_data_lines(&mut buf, &[*byte]).is_empty());
        }
        assert_eq!(
            extract_sse_data_lines(&mut buf, b"\n"),
            vec![b"fragmented".to_vec()]
        );
        assert!(buf.is_empty());
    }

    #[test]
    fn consumed_large_comment_releases_scratch_capacity_before_reuse() {
        const LARGE_COMMENT_BYTES: usize = 4 * 1024 * 1024;
        const RETAINED_SCRATCH_MAX: usize =
            crate::client::transport::limits::SSE_PARSE_SLICE_BYTES * 2;

        let mut input = Vec::with_capacity(LARGE_COMMENT_BYTES + 3);
        input.push(b':');
        input.resize(LARGE_COMMENT_BYTES + 1, b'x');
        input.extend_from_slice(b"\ndata: pending");

        let mut parser = SseEventParser::new();
        assert!(parser.try_push(&input).unwrap().is_empty());
        assert_eq!(parser.buffered_len(), b"data: pending".len());
        assert!(
            parser.buf.capacity() <= RETAINED_SCRATCH_MAX,
            "consumed comment retained {} bytes of scratch capacity",
            parser.buf.capacity(),
        );

        assert_eq!(parser.try_push(b"\n\n").unwrap(), vec![b"pending".to_vec()],);
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
    fn event_parser_preserves_fragmented_empty_data_lines_and_mixed_endings() {
        let mut parser = SseEventParser::new();
        let fragments: [&[u8]; 8] = [
            b"da",
            b"ta:\r",
            b"\nda",
            b"ta: {\"a\":\r",
            b"data: 1}\n",
            b"data:\r",
            b"\n\r",
            b"\n",
        ];

        for fragment in &fragments[..fragments.len() - 1] {
            assert!(parser.try_push(fragment).unwrap().is_empty());
        }
        assert_eq!(
            parser.try_push(fragments[fragments.len() - 1]).unwrap(),
            vec![b"\n{\"a\":\n1}\n".to_vec()]
        );
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
    fn event_parser_accepts_lone_cr_line_endings() {
        let mut parser = SseEventParser::new();
        assert_eq!(
            parser.push(b"data: first\r\rdata: second\r\r:"),
            vec![b"first".to_vec(), b"second".to_vec()]
        );
    }

    #[test]
    fn event_parser_ignores_a_fragmented_utf8_bom() {
        let mut parser = SseEventParser::new();
        assert!(parser.push(b"\xef").is_empty());
        assert!(parser.push(b"\xbb").is_empty());
        assert_eq!(
            parser.push(b"\xbfdata: payload\n\n"),
            vec![b"payload".to_vec()]
        );
    }

    #[test]
    fn bounded_parser_rejects_payload_and_line_count_before_dispatch() {
        let mut oversized = SseEventParser::new();
        oversized.feed(b"data: 12345\n\n");
        let error = oversized
            .next_event_with_limits(4, usize::MAX, false)
            .unwrap_err();
        assert_eq!(error.code(), Some(codes::SDK_VALIDATION));
        assert!(error.message().contains("exceeded limit"));

        let mut too_many_lines = SseEventParser::new();
        too_many_lines.feed(b"data: 1\ndata: 2\n\n");
        let error = too_many_lines
            .next_event_with_limits(usize::MAX, 1, false)
            .unwrap_err();
        assert_eq!(error.code(), Some(codes::SDK_VALIDATION));
        assert!(error.message().contains("data-line limit"));
    }

    #[test]
    fn bounded_parser_counts_inserted_newlines_in_the_byte_limit() {
        let mut exact = SseEventParser::new();
        exact.feed(b"data: a\ndata: b\n\n");
        assert_eq!(
            exact.next_event_with_limits(3, 2, false).unwrap(),
            Some(b"a\nb".to_vec())
        );

        let mut oversized = SseEventParser::new();
        oversized.feed(b"data: a\ndata: b\n\n");
        let error = oversized.next_event_with_limits(2, 2, false).unwrap_err();
        assert_eq!(error.code(), Some(codes::SDK_VALIDATION));
        assert!(error.message().contains("exceeded limit"));
    }

    #[test]
    fn public_parser_accepts_exact_data_line_limit_with_empty_payloads() {
        let line_limit = crate::client::transport::limits::SSE_EVENT_DATA_LINES_MAX;
        let mut input = b"data:\n".repeat(line_limit);
        input.push(b'\n');

        let mut parser = SseEventParser::new();
        let events = parser.try_push(&input).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].len(), line_limit - 1);
        assert!(events[0].iter().all(|byte| *byte == b'\n'));
    }

    #[test]
    fn public_parser_limits_line_count_and_releases_failed_event() {
        let mut input = Vec::new();
        for _ in 0..=crate::client::transport::limits::SSE_EVENT_DATA_LINES_MAX {
            input.extend_from_slice(b"data: x\n");
        }

        let mut parser = SseEventParser::new();
        let error = parser.try_push(&input).unwrap_err();
        assert_eq!(error.code(), Some(codes::SDK_VALIDATION));
        assert!(error.message().contains("data-line limit"));
        assert_eq!(parser.buffered_len(), 0);
        assert_eq!(parser.buf.capacity(), 0);
        assert_eq!(parser.event_data.capacity(), 0);

        // The compatibility API also fails closed instead of retaining an
        // attacker-controlled event indefinitely.
        assert!(parser.push(&input).is_empty());
        assert_eq!(parser.buffered_len(), 0);

        // A reset parser remains useful after the caller handles the failure.
        assert_eq!(
            parser.try_push(b"data: recovered\n\n").unwrap(),
            vec![b"recovered".to_vec()]
        );
    }

    #[test]
    fn exact_byte_limit_is_independent_of_transport_terminator_chunk() {
        let payload_len = crate::client::transport::limits::SSE_EVENT_BYTES_MAX;
        let mut input = Vec::with_capacity(payload_len + 6);
        input.extend_from_slice(b"data: ");
        input.resize(payload_len + 6, b'x');

        let mut parser = SseEventParser::new();
        assert!(parser.try_push(&input).unwrap().is_empty());
        drop(input);

        let events = parser.try_push(b"\n\n").unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].len(), payload_len);
        assert_eq!(events[0].first(), Some(&b'x'));
        assert_eq!(events[0].last(), Some(&b'x'));
    }

    #[test]
    fn bounded_parser_returns_one_event_at_a_time() {
        let mut parser = SseEventParser::new();
        parser.feed(b"data: first\n\ndata: second\n\n");

        assert_eq!(parser.next_bounded().unwrap(), Some(b"first".to_vec()));
        assert_eq!(parser.next_bounded().unwrap(), Some(b"second".to_vec()));
        assert_eq!(parser.next_bounded().unwrap(), None);
        assert_eq!(parser.buffered_len(), 0);
    }

    #[test]
    fn incomplete_line_scanning_advances_only_over_new_bytes() {
        let mut parser = SseEventParser::new();
        parser.feed(b"data: a long line without a terminator");
        assert_eq!(parser.next_bounded().unwrap(), None);
        assert_eq!(parser.search_from, parser.buf.len());

        let previous_len = parser.buf.len();
        parser.feed(b" and a little more");
        assert_eq!(parser.next_bounded().unwrap(), None);
        assert_eq!(parser.search_from, parser.buf.len());
        assert!(parser.search_from > previous_len);

        // A trailing CR is the sole byte that must be revisited so a CRLF split
        // across chunks is still recognized as one terminator.
        parser.feed(b"\r");
        assert_eq!(parser.next_bounded().unwrap(), None);
        assert_eq!(parser.search_from, parser.buf.len() - 1);
        parser.feed(b"\n\r\n");
        assert!(parser.next_bounded().unwrap().is_some());
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

    #[tokio::test]
    async fn required_done_stream_handles_transport_fragmentation() {
        let raw: crate::client::transport::SseByteStream = Box::pin(futures_util::stream::iter([
            Ok(Bytes::from_static(b"data: {\"value\":")),
            Ok(Bytes::from_static(b"1}\n\ndata: [DO")),
            Ok(Bytes::from_static(b"NE]\n\n")),
        ]));
        let mut stream = decode_required_done_stream(raw, |payload| {
            serde_json::from_slice::<serde_json::Value>(payload).map_err(ZaiError::from)
        });
        assert_eq!(stream.next().await.unwrap().unwrap()["value"], 1);
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn required_done_stream_accepts_exact_limit_before_later_terminator() {
        let payload_len = crate::client::transport::limits::SSE_EVENT_BYTES_MAX;
        let mut first_chunk = Vec::with_capacity(payload_len + 6);
        first_chunk.extend_from_slice(b"data: ");
        first_chunk.resize(payload_len + 6, b'x');

        let raw: crate::client::transport::SseByteStream = Box::pin(futures_util::stream::iter([
            Ok(Bytes::from(first_chunk)),
            Ok(Bytes::from_static(b"\n\ndata: [DONE]\n\n")),
        ]));
        let mut stream = decode_required_done_stream(raw, |payload| Ok(payload.len()));

        assert_eq!(stream.next().await.unwrap().unwrap(), payload_len);
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn required_done_stream_accepts_bom_and_lone_cr_events() {
        let raw: crate::client::transport::SseByteStream =
            Box::pin(futures_util::stream::iter([Ok(Bytes::from_static(
                b"\xef\xbb\xbfdata: {\"value\":1}\r\rdata: [DONE]\r\r:",
            ))]));
        let mut stream = decode_required_done_stream(raw, |payload| {
            serde_json::from_slice::<serde_json::Value>(payload).map_err(ZaiError::from)
        });
        assert_eq!(stream.next().await.unwrap().unwrap()["value"], 1);
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn required_done_stream_reports_missing_marker_once() {
        let raw: crate::client::transport::SseByteStream =
            Box::pin(futures_util::stream::iter([Ok(Bytes::from_static(
                b"data: {\"value\":1}\n\n",
            ))]));
        let mut stream = decode_required_done_stream(raw, |payload| {
            serde_json::from_slice::<serde_json::Value>(payload).map_err(ZaiError::from)
        });
        assert!(stream.next().await.unwrap().is_ok());
        let error = stream.next().await.unwrap().unwrap_err();
        assert_eq!(error.code(), Some(codes::SDK_IO));
        assert!(error.message().contains("[DONE]"));
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn required_done_stream_terminates_after_decode_or_business_error() {
        for body in [
            b"data: not-json\n\ndata: {\"value\":1}\n\ndata: [DONE]\n\n".as_slice(),
            b"data: {\"error\":{\"code\":1302,\"message\":\"limited\"}}\n\ndata: [DONE]\n\n"
                .as_slice(),
        ] {
            let raw: crate::client::transport::SseByteStream = Box::pin(
                futures_util::stream::iter([Ok(Bytes::copy_from_slice(body))]),
            );
            let mut stream = decode_required_done_stream(raw, |payload| {
                serde_json::from_slice::<serde_json::Value>(payload).map_err(ZaiError::from)
            });
            assert!(stream.next().await.unwrap().is_err());
            assert!(stream.next().await.is_none());
        }
    }

    #[tokio::test]
    async fn terminal_decode_errors_drop_the_raw_stream_before_being_yielded() {
        struct DropFlag(Arc<AtomicBool>);
        impl Drop for DropFlag {
            fn drop(&mut self) {
                self.0.store(true, Ordering::SeqCst);
            }
        }

        for body in [
            Bytes::from_static(b"data: not-json\n\n"),
            Bytes::from_static(b"data: {\"error\":{\"code\":1302,\"message\":\"limited\"}}\n\n"),
        ] {
            let dropped = Arc::new(AtomicBool::new(false));
            let guard = DropFlag(Arc::clone(&dropped));
            let mut body = Some(body);
            let raw: crate::client::transport::SseByteStream =
                Box::pin(futures_util::stream::poll_fn(move |_| {
                    let _keep_guard_alive = &guard;
                    match body.take() {
                        Some(body) => std::task::Poll::Ready(Some(Ok(body))),
                        None => std::task::Poll::Pending,
                    }
                }));
            let mut stream = decode_required_done_stream(raw, |payload| {
                serde_json::from_slice::<serde_json::Value>(payload).map_err(ZaiError::from)
            });

            assert!(stream.next().await.unwrap().is_err());
            assert!(
                dropped.load(Ordering::SeqCst),
                "terminal errors must release the raw response before the caller polls again"
            );
            // Keep `stream` alive through the assertion: release must not rely
            // on Drop or a follow-up poll.
            assert!(stream.next().await.is_none());
        }
    }

    #[tokio::test]
    async fn ambiguous_in_band_envelopes_error_once_drop_raw_and_do_not_leak() {
        struct DropFlag(Arc<AtomicBool>);
        impl Drop for DropFlag {
            fn drop(&mut self) {
                self.0.store(true, Ordering::SeqCst);
            }
        }

        for (case, payload, secret) in [
            (
                "top-level code",
                r#"{"id":"valid-chat-like","choices":[],"code":1302,"code":200,"message":"private-parser-top-level"}"#,
                "private-parser-top-level",
            ),
            (
                "nested error code",
                r#"{"id":"valid-asr-like","type":"transcript.text.delta","delta":"valid","error":{"code":1302,"code":200,"message":"private-parser-nested"}}"#,
                "private-parser-nested",
            ),
        ] {
            let body = Bytes::from(format!(
                "data: {payload}\n\ndata: {{\"id\":\"after-error\"}}\n\ndata: [DONE]\n\n"
            ));
            let dropped = Arc::new(AtomicBool::new(false));
            let guard = DropFlag(Arc::clone(&dropped));
            let mut body = Some(body);
            let raw: crate::client::transport::SseByteStream =
                Box::pin(futures_util::stream::poll_fn(move |_| {
                    let _keep_guard_alive = &guard;
                    match body.take() {
                        Some(body) => std::task::Poll::Ready(Some(Ok(body))),
                        None => std::task::Poll::Pending,
                    }
                }));
            let mut stream = decode_required_done_stream(raw, |payload| {
                serde_json::from_slice::<serde_json::Value>(payload).map_err(ZaiError::from)
            });

            let error = stream.next().await.unwrap().unwrap_err();
            assert_eq!(error.code(), Some(codes::SDK_VALIDATION), "{case}");
            assert_eq!(
                error.message(),
                "ambiguous JSON business-error envelope (duplicate reserved field)",
                "{case}"
            );
            assert!(
                dropped.load(Ordering::SeqCst),
                "{case}: ambiguous error did not immediately release raw response"
            );
            for rendered in [error.to_string(), format!("{error:?}"), error.compact()] {
                assert!(!rendered.contains(secret), "{case}: {rendered}");
                assert!(!rendered.contains("1302"), "{case}: {rendered}");
            }
            assert!(stream.next().await.is_none(), "{case}");
        }
    }
}
