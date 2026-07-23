//! Local scripted WebSocket server used by realtime integration tests.
//!
//! A `WsTestServer` binds `127.0.0.1:0`, captures the upgrade request
//! (path + headers) for authentication assertions, pushes a scripted queue of
//! frames to the client in FIFO order, and records every inbound client frame
//! for later assertions. It supports handshake, event-roundtrip, error and
//! close-semantics tests without touching the real Zhipu realtime API.
//!
//! This is a test-support module (only compiled for `cfg(test)` / integration
//! tests); it lives under `tests/support/`.

// Each integration-test binary compiles this module independently and uses
// only a subset of its API, so per-binary dead-code analysis would otherwise
// flag the unused remainder.
#![allow(dead_code)]

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};

/// A captured upgrade (handshake) request.
#[derive(Debug, Clone)]
pub struct CapturedHandshake {
    pub path: String,
    pub authorization: Option<String>,
    pub headers: Vec<(String, String)>,
}

/// A captured inbound client frame.
#[derive(Debug, Clone)]
pub enum CapturedFrame {
    Text(String),
    Binary(Vec<u8>),
}

impl CapturedFrame {
    /// The text payload, when the frame is a text frame.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            Self::Binary(_) => None,
        }
    }
}

/// One scripted outbound frame. The queue is consumed FIFO as soon as the
/// upgrade completes; a `Close` entry starts the close handshake after all
/// preceding frames have been sent.
#[derive(Debug, Clone)]
pub enum ScriptedFrame {
    Text(String),
    Binary(Vec<u8>),
    Close,
}

impl ScriptedFrame {
    /// A text frame carrying a serialized JSON event.
    pub fn json(body: serde_json::Value) -> Self {
        Self::Text(body.to_string())
    }
}

/// A local scripted WebSocket server bound to `127.0.0.1:0`.
pub struct WsTestServer {
    pub url: String,
    handshakes: Arc<Mutex<Vec<CapturedHandshake>>>,
    received: Arc<Mutex<Vec<CapturedFrame>>>,
    shutdown: Arc<tokio::sync::Notify>,
}

impl WsTestServer {
    /// Start a server that accepts connections and serves `script` FIFO.
    pub async fn start(script: Vec<ScriptedFrame>) -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handshakes: Arc<Mutex<Vec<CapturedHandshake>>> = Arc::new(Mutex::new(Vec::new()));
        let received: Arc<Mutex<Vec<CapturedFrame>>> = Arc::new(Mutex::new(Vec::new()));
        let queue: Arc<Mutex<VecDeque<ScriptedFrame>>> = Arc::new(Mutex::new(script.into()));
        let shutdown = Arc::new(tokio::sync::Notify::new());

        let accept_handshakes = Arc::clone(&handshakes);
        let accept_received = Arc::clone(&received);
        let accept_queue = Arc::clone(&queue);
        let accept_shutdown = Arc::clone(&shutdown);
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    biased;
                    () = accept_shutdown.notified() => break,
                    accept = listener.accept() => {
                        let (stream, _) = match accept { Ok(s) => s, Err(_) => continue };
                        let handshakes = Arc::clone(&accept_handshakes);
                        let received = Arc::clone(&accept_received);
                        let queue = Arc::clone(&accept_queue);
                        tokio::spawn(run_connection(stream, handshakes, received, queue));
                    }
                }
            }
        });

        Self {
            url: format!("ws://{addr}"),
            handshakes,
            received,
            shutdown,
        }
    }

    /// All upgrade requests captured so far (FIFO).
    pub fn handshakes(&self) -> Vec<CapturedHandshake> {
        self.handshakes.lock().unwrap().clone()
    }

    /// All client frames captured so far (FIFO).
    pub fn received(&self) -> Vec<CapturedFrame> {
        self.received.lock().unwrap().clone()
    }

    /// Wait until at least `count` client frames have been captured, then
    /// return them. Panics after five seconds so a stuck client fails the
    /// test loudly instead of hanging the test binary.
    pub async fn wait_for_frames(&self, count: usize) -> Vec<CapturedFrame> {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            let frames = self.received();
            if frames.len() >= count {
                return frames;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "timed out waiting for {count} client frame(s)"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    /// Shut the server down.
    pub async fn shutdown(&self) {
        // `notify_one` stores a permit when the accept loop is between polls;
        // `notify_waiters` could lose the shutdown signal in that window.
        self.shutdown.notify_one();
    }
}

/// Serve one accepted connection: capture the upgrade request, push the
/// scripted frames, then record inbound frames until the peer closes or the
/// socket fails. Connection tasks are not tied to the server shutdown
/// signal: every test closes its session, which ends the connection, and a
/// panicking test drops the client socket with the same effect.
// `result_large_err`: the handshake callback's `Result<Response, ErrorResponse>`
// signature is dictated by `accept_hdr_async` and cannot be boxed down.
#[allow(clippy::result_large_err)]
async fn run_connection(
    stream: TcpStream,
    handshakes: Arc<Mutex<Vec<CapturedHandshake>>>,
    received: Arc<Mutex<Vec<CapturedFrame>>>,
    queue: Arc<Mutex<VecDeque<ScriptedFrame>>>,
) {
    let capture = move |request: &Request, response: Response| {
        let authorization = request
            .headers()
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let headers = request
            .headers()
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_str().unwrap_or("").to_string()))
            .collect();
        handshakes.lock().unwrap().push(CapturedHandshake {
            path: request.uri().path().to_string(),
            authorization,
            headers,
        });
        Ok(response)
    };
    let mut socket = match tokio_tungstenite::accept_hdr_async(stream, capture).await {
        Ok(socket) => socket,
        // A failed upgrade cannot serve the test; drop the connection.
        Err(_) => return,
    };

    // Push the script FIFO. A scripted `Close` starts the close handshake but
    // the read loop below keeps running so late client frames are still
    // recorded.
    loop {
        let next = queue.lock().unwrap().pop_front();
        match next {
            Some(ScriptedFrame::Text(text)) => {
                if socket.send(Message::Text(text.into())).await.is_err() {
                    return;
                }
            },
            Some(ScriptedFrame::Binary(bytes)) => {
                if socket.send(Message::Binary(bytes.into())).await.is_err() {
                    return;
                }
            },
            Some(ScriptedFrame::Close) => {
                let _ = socket.close(None).await;
                break;
            },
            None => break,
        }
    }

    loop {
        match socket.next().await {
            None | Some(Err(_)) => break,
            Some(Ok(Message::Text(text))) => {
                received
                    .lock()
                    .unwrap()
                    .push(CapturedFrame::Text(text.to_string()));
            },
            Some(Ok(Message::Binary(bytes))) => {
                received
                    .lock()
                    .unwrap()
                    .push(CapturedFrame::Binary(bytes.to_vec()));
            },
            // Answer a client close so the peer's graceful close completes
            // instead of running into its close timeout.
            Some(Ok(Message::Close(_))) => {
                let _ = socket.close(None).await;
                break;
            },
            Some(Ok(_)) => {},
        }
    }
}
