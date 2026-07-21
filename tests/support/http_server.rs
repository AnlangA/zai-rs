//! Local scripted HTTP server used by integration tests.
//!
//! A `TestServer` binds `127.0.0.1:0` and serves a scripted queue of responses
//! to support transport, retry and redaction tests without touching the real
//! Zhipu API. It serves fixed bodies and captures requests for later
//! assertions.
//!
//! This is a test-support module (only compiled for `cfg(test)` / integration
//! tests); it lives under `tests/support/`.

// Each integration-test binary compiles this module independently and uses
// only a subset of its API, so per-binary dead-code analysis would otherwise
// flag the unused remainder.
#![allow(dead_code)]

use std::convert::Infallible;
use std::sync::{Arc, Mutex};

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::http::StatusCode;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder as ConnBuilder;
use tokio::net::TcpListener;

/// A captured inbound request.
#[derive(Debug, Clone)]
pub struct CapturedRequest {
    pub method: String,
    pub path: String,
    pub query: Option<String>,
    pub authorization: Option<String>,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// One scripted response. The queue is consumed FIFO; running out of responses
/// serves a 599 "no more scripted responses" so the test fails loudly.
#[derive(Clone)]
pub struct ScriptedResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Bytes,
}

impl ScriptedResponse {
    /// A simple JSON 200 response.
    pub fn json(status: u16, body: serde_json::Value) -> Self {
        Self::raw(status, "application/json", Bytes::from(body.to_string()))
    }

    /// A response with the given status and a raw body + content type.
    pub fn raw(status: u16, content_type: &str, body: impl Into<Bytes>) -> Self {
        Self {
            status,
            headers: vec![("content-type".into(), content_type.into())],
            body: body.into(),
        }
    }

    /// An empty response with no explicit content type (e.g. a bare 500).
    pub fn empty(status: u16) -> Self {
        Self {
            status,
            headers: vec![],
            body: Bytes::new(),
        }
    }
}

/// A local test server bound to `127.0.0.1:0`.
pub struct TestServer {
    pub base_url: String,
    pub captured: Arc<Mutex<Vec<CapturedRequest>>>,
    shutdown: Arc<tokio::sync::Notify>,
}

impl TestServer {
    /// Start a server that serves `responses` in FIFO order.
    pub async fn start(responses: Vec<ScriptedResponse>) -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let addr = listener.local_addr().unwrap();
        let captured: Arc<Mutex<Vec<CapturedRequest>>> = Arc::new(Mutex::new(Vec::new()));
        let queue: Arc<Mutex<std::collections::VecDeque<ScriptedResponse>>> =
            Arc::new(Mutex::new(responses.into()));
        let shutdown = Arc::new(tokio::sync::Notify::new());

        let captured_task = captured.clone();
        let queue_task = queue.clone();
        let shutdown_task = shutdown.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    biased;
                    () = shutdown_task.notified() => break,
                    accept = listener.accept() => {
                        let (stream, _) = match accept { Ok(s) => s, Err(_) => continue };
                        let io = TokioIo::new(stream);
                        let captured = captured_task.clone();
                        let queue = queue_task.clone();
                        tokio::spawn(async move {
                            let service = service_fn(move |req: Request<Incoming>| {
                                let captured = captured.clone();
                                let queue = queue.clone();
                                async move {
                                    let method = req.method().as_str().to_string();
                                    let uri = req.uri().clone();
                                    let authorization = header_opt(&req, "authorization");
                                    let all_headers = req
                                        .headers()
                                        .iter()
                                        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                                        .collect::<Vec<_>>();
                                    let body = req.collect().await.unwrap().to_bytes().to_vec();
                                    captured.lock().unwrap().push(CapturedRequest {
                                        method,
                                        path: uri.path().to_string(),
                                        query: uri.query().map(str::to_string),
                                        authorization,
                                        headers: all_headers,
                                        body,
                                    });

                                    let next = queue.lock().unwrap().pop_front();
                                    let resp = match next {
                                        None => Response::builder()
                                            .status(StatusCode::from_u16(599).unwrap())
                                            .body(Full::new(Bytes::from_static(
                                                b"no more scripted responses",
                                            )))
                                            .unwrap(),
                                        Some(s) => {
                                            let status =
                                                StatusCode::from_u16(s.status).unwrap_or_else(|_| {
                                                    StatusCode::from_u16(500).unwrap()
                                                });
                                            let mut builder = Response::builder().status(status);
                                            for (k, v) in &s.headers {
                                                builder = builder.header(k.as_str(), v.as_str());
                                            }
                                            builder.body(Full::new(s.body)).unwrap()
                                        }
                                    };
                                    Ok::<_, Infallible>(resp)
                                }
                            });
                            ConnBuilder::new(TokioExecutor::new())
                                .serve_connection(io, service)
                                .await
                                .unwrap();
                        });
                    }
                }
            }
        });

        Self {
            base_url: format!("http://{addr}"),
            captured,
            shutdown,
        }
    }

    /// All requests captured so far (FIFO).
    pub fn requests(&self) -> Vec<CapturedRequest> {
        self.captured.lock().unwrap().clone()
    }

    /// Shut the server down.
    pub async fn shutdown(&self) {
        // `notify_one` stores a permit when the accept loop is between polls;
        // `notify_waiters` could lose the shutdown signal in that window.
        self.shutdown.notify_one();
    }
}

fn header_opt(req: &Request<Incoming>, name: &str) -> Option<String> {
    req.headers()
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
}
