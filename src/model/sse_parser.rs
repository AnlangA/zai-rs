//! Shared SSE (Server-Sent Events) line parsing utilities.
//!
//! Extracts the common logic of buffering raw byte chunks, splitting on `\n`,
//! trimming `\r\n`, and yielding `data: ` prefixed payload lines.

/// Process a new chunk of bytes, extract completed SSE data lines.
///
/// Appends `new_bytes` to `buf`, then extracts all complete lines (delimited
/// by `\n`). For each line, it:
/// - Strips trailing `\r` and `\n`
/// - Skips empty lines
/// - Strips the `"data: "` prefix and yields the remaining bytes
///
/// Returns a vector of data payload slices (borrowed from `buf`).
/// Lines that are not prefixed with `"data: "` are silently skipped.
///
/// If a `data: [DONE]` line is encountered, it is yielded as a
/// `[b"[DONE]"]` entry so the caller can detect stream termination.
pub fn extract_sse_data_lines(buf: &mut Vec<u8>, new_bytes: &[u8]) -> Vec<Vec<u8>> {
    buf.extend_from_slice(new_bytes);
    let mut results = Vec::new();

    while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
        // Extract line including the '\n', leaving remaining bytes in buf
        let line_with_nl = buf.drain(..=pos).collect::<Vec<u8>>();

        // Trim trailing \n and \r
        let mut line = &line_with_nl[..];
        if line.ends_with(b"\n") {
            line = &line[..line.len() - 1];
        }
        if line.ends_with(b"\r") {
            line = &line[..line.len() - 1];
        }
        if line.is_empty() {
            continue;
        }
        const PREFIX: &[u8] = b"data: ";
        if line.starts_with(PREFIX) {
            let rest = &line[PREFIX.len()..];
            results.push(rest.to_vec());
        }
    }

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
}
