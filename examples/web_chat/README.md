# zai-rs web chat

A deliberately small Axum application that demonstrates how to share one
`ZaiClient` across browser chat requests. It includes bounded in-memory
conversation history, per-peer rate limiting, validated configuration, health
probes, and a dependency-free browser UI.

## Run

From the repository root:

```shell
export ZHIPU_API_KEY="your-key"
cargo run -p web_chat
```

Open <http://127.0.0.1:3000>. The listener binds to loopback by default.

## Configuration

| Variable | Default | Meaning |
| --- | --- | --- |
| `ZHIPU_API_KEY` | required | Z.AI API key; never logged |
| `BIND_ADDRESS` | `127.0.0.1` | Listener IP address |
| `PORT` | `3000` | Listener port |
| `CORS_ORIGINS` | local URLs using `PORT` | Comma-separated allowed origins |
| `SESSION_TIMEOUT` | `3600` | Idle session lifetime in seconds |
| `MAX_MESSAGES_PER_SESSION` | `1000` | Hard cap on retained messages |
| `RUST_LOG` | `info` | Standard tracing filter |

Invalid values fail startup before the listener opens.

## HTTP surface

- `POST /api/chat/send` returns a JSON response.
- `POST /api/chat/stream` relays typed model deltas as SSE and emits a final
  `done` event only after the provider's `[DONE]` marker and successful history
  update. Partial assistant text is not retained if the stream fails or the
  browser disconnects.
- `GET /api/chat/history/{session_id}` returns retained conversation history.
- `POST /api/chat/clear/{session_id}` clears retained history.
- `GET /health`, `/ready`, and `/live` expose process probes.

Each SSE `data` frame is a JSON object containing `content`, `session_id`, and
`done`, with optional `error`, `metadata`, and `usage` fields. Errors are
terminal frames (`done: true`); a successful terminal frame is sent only after
the complete assistant response has been stored.

Requests sharing a session ID are serialized for the lifetime of the upstream
request, while independent sessions remain concurrent. The session store is
process-local and intentionally not suitable for multi-instance production
deployment.

This example has no user authentication and should not be exposed directly to
an untrusted network. Reverse-proxy deployments must add authentication and
should replace direct peer-IP rate limiting with an explicitly trusted proxy
setup.
