# Seed corpora

`cargo fuzz run <target>` uses the matching directory below as its input
corpus. Only small, reviewed files named `seed-*` are committed; hash-named
inputs discovered by libFuzzer remain ignored.

- `fuzz_sse_decoder`: byte 0 selects a chunk size from 1 through 64; the
  remaining bytes are an SSE stream.
- `fuzz_error_handling`: bytes 0–1 are a big-endian HTTP status, bytes 2–3 are
  a big-endian API code, and the entire lossy UTF-8 input is also redacted.
- `fuzz_url_segments`: valid UTF-8 input split on NUL bytes supplies up to eight
  URL path segments.

All credential-like values are synthetic placeholders.
