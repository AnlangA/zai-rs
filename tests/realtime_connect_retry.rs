//! End-to-end recovery tests for the built-in realtime connection path.
#![cfg(feature = "realtime")]

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};

use base64::Engine as _;
use futures_util::StreamExt as _;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{
    accept_hdr_async,
    tungstenite::{
        Message,
        handshake::server::{ErrorResponse, Request, Response},
        http::{
            StatusCode,
            header::{CONTENT_LENGTH, RETRY_AFTER},
        },
    },
};
use zai_rs::{
    ZaiError,
    client::{EndpointConfig, error::RealtimeErrorKind},
    model::GLM_realtime_flash,
    realtime::{RealtimeClient, RealtimeTransportConfig, TungsteniteTransport},
};

const TEST_KEY: &str = "test.12345678901234567890";

#[derive(Clone)]
enum HandshakeOutcome {
    Reject {
        status: StatusCode,
        body: &'static str,
        retry_after: Option<&'static str>,
        response_header: Option<(&'static str, &'static str)>,
    },
    Accept,
}

#[derive(Debug, Clone)]
struct CapturedHandshake {
    authorization: Option<String>,
}

struct HandshakeServer {
    url: String,
    handshakes: Arc<Mutex<Vec<CapturedHandshake>>>,
    frames: Arc<Mutex<Vec<String>>>,
    shutdown: Arc<tokio::sync::Notify>,
}

impl HandshakeServer {
    async fn start(outcomes: Vec<HandshakeOutcome>) -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let addr = listener.local_addr().unwrap();
        let outcomes = Arc::new(Mutex::new(VecDeque::from(outcomes)));
        let handshakes = Arc::new(Mutex::new(Vec::new()));
        let frames = Arc::new(Mutex::new(Vec::new()));
        let shutdown = Arc::new(tokio::sync::Notify::new());

        let accept_outcomes = Arc::clone(&outcomes);
        let accept_handshakes = Arc::clone(&handshakes);
        let accept_frames = Arc::clone(&frames);
        let accept_shutdown = Arc::clone(&shutdown);
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    biased;
                    () = accept_shutdown.notified() => break,
                    accepted = listener.accept() => {
                        let Ok((stream, _)) = accepted else { continue };
                        let outcome = accept_outcomes
                            .lock()
                            .unwrap()
                            .pop_front()
                            .unwrap_or(HandshakeOutcome::Reject {
                                status: StatusCode::SERVICE_UNAVAILABLE,
                                body: "unexpected extra handshake",
                                retry_after: None,
                                response_header: None,
                            });
                        let handshakes = Arc::clone(&accept_handshakes);
                        let frames = Arc::clone(&accept_frames);
                        tokio::spawn(serve_connection(stream, outcome, handshakes, frames));
                    }
                }
            }
        });

        Self {
            url: format!("ws://{addr}"),
            handshakes,
            frames,
            shutdown,
        }
    }

    fn handshakes(&self) -> Vec<CapturedHandshake> {
        self.handshakes.lock().unwrap().clone()
    }

    async fn wait_for_frames(&self, count: usize) -> Vec<String> {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            let frames = self.frames.lock().unwrap().clone();
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

    async fn shutdown(&self) {
        self.shutdown.notify_one();
    }
}

async fn serve_connection(
    stream: TcpStream,
    outcome: HandshakeOutcome,
    handshakes: Arc<Mutex<Vec<CapturedHandshake>>>,
    frames: Arc<Mutex<Vec<String>>>,
) {
    // `accept_hdr_async` fixes this callback's error type to the full HTTP
    // response so a test server can exercise status, headers, and body.
    #[allow(clippy::result_large_err)]
    let callback = move |request: &Request, response: Response| {
        let authorization = request
            .headers()
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        handshakes
            .lock()
            .unwrap()
            .push(CapturedHandshake { authorization });

        match outcome {
            HandshakeOutcome::Accept => Ok(response),
            HandshakeOutcome::Reject {
                status,
                body,
                retry_after,
                response_header,
            } => {
                let mut rejected = ErrorResponse::new(Some(body.to_owned()));
                *rejected.status_mut() = status;
                rejected
                    .headers_mut()
                    .insert(CONTENT_LENGTH, body.len().to_string().parse().unwrap());
                if let Some(value) = retry_after {
                    rejected
                        .headers_mut()
                        .insert(RETRY_AFTER, value.parse().unwrap());
                }
                if let Some((name, value)) = response_header {
                    rejected.headers_mut().insert(name, value.parse().unwrap());
                }
                Err(rejected)
            },
        }
    };

    let Ok(mut socket) = accept_hdr_async(stream, callback).await else {
        return;
    };
    while let Some(message) = socket.next().await {
        match message {
            Ok(Message::Text(text)) => frames.lock().unwrap().push(text.to_string()),
            Ok(Message::Close(_)) => {
                let _ = socket.close(None).await;
                break;
            },
            Err(_) => break,
            _ => {},
        }
    }
}

fn client_for(server: &HandshakeServer, config: RealtimeTransportConfig) -> RealtimeClient {
    let endpoints = EndpointConfig::builder()
        .realtime(format!("{}/realtime", server.url))
        .build(true)
        .unwrap();
    RealtimeClient::new(TEST_KEY)
        .with_endpoint_config(endpoints)
        .with_transport_config(config)
}

fn jwt_timestamp(authorization: &str) -> i64 {
    let token = authorization.strip_prefix("Bearer ").unwrap();
    let payload = token.split('.').nth(1).unwrap();
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .unwrap();
    serde_json::from_slice::<serde_json::Value>(&decoded).unwrap()["timestamp"]
        .as_i64()
        .unwrap()
}

#[tokio::test]
async fn retryable_handshake_refreshes_jwt_and_sends_one_initial_update() {
    let server = HandshakeServer::start(vec![
        HandshakeOutcome::Reject {
            status: StatusCode::SERVICE_UNAVAILABLE,
            body: r#"{"message":"temporarily unavailable"}"#,
            retry_after: Some("1"),
            response_header: None,
        },
        HandshakeOutcome::Accept,
    ])
    .await;
    let config = RealtimeTransportConfig::builder()
        .connect_timeout(Duration::from_secs(3))
        .max_connect_attempts(2)
        .try_build()
        .unwrap();

    let session = client_for(&server, config)
        .with_jwt(1)
        .session(GLM_realtime_flash {})
        .build()
        .await
        .unwrap();

    let handshakes = server.handshakes();
    assert_eq!(handshakes.len(), 2);
    let first = handshakes[0].authorization.as_deref().unwrap();
    let second = handshakes[1].authorization.as_deref().unwrap();
    assert_ne!(
        first, second,
        "each connection attempt must sign a fresh JWT"
    );
    assert!(jwt_timestamp(second) > jwt_timestamp(first));

    let frames = server.wait_for_frames(1).await;
    assert_eq!(frames.len(), 1);
    let init: serde_json::Value = serde_json::from_str(&frames[0]).unwrap();
    assert_eq!(init["type"], "session.update");
    assert_eq!(init["session"]["model"], "glm-realtime-flash");

    session.close().await.unwrap();
    server.shutdown().await;
}

#[tokio::test]
async fn authentication_handshake_is_not_retried() {
    let server = HandshakeServer::start(vec![HandshakeOutcome::Reject {
        status: StatusCode::UNAUTHORIZED,
        body: r#"{"message":"invalid credential"}"#,
        retry_after: None,
        response_header: None,
    }])
    .await;
    let config = RealtimeTransportConfig::builder()
        .connect_timeout(Duration::from_secs(1))
        .max_connect_attempts(3)
        .try_build()
        .unwrap();

    let error = client_for(&server, config)
        .session(GLM_realtime_flash {})
        .build()
        .await
        .err()
        .expect("401 handshake unexpectedly connected");

    assert!(error.message().contains("401"), "unexpected error: {error}");
    assert!(error.is_auth_error());
    assert!(!error.is_retryable());
    assert_eq!(server.handshakes().len(), 1);
    server.shutdown().await;
}

#[tokio::test]
async fn direct_transport_connect_remains_single_attempt() {
    let server = HandshakeServer::start(vec![
        HandshakeOutcome::Reject {
            status: StatusCode::SERVICE_UNAVAILABLE,
            body: r#"{"message":"temporarily unavailable"}"#,
            retry_after: None,
            response_header: None,
        },
        HandshakeOutcome::Accept,
    ])
    .await;
    let config = RealtimeTransportConfig::builder()
        .connect_timeout(Duration::from_secs(1))
        .max_connect_attempts(3)
        .try_build()
        .unwrap();

    let error = TungsteniteTransport::connect_with_config(
        &format!("{}/realtime", server.url),
        "Bearer direct-test-token",
        config,
    )
    .await
    .err()
    .expect("direct 503 handshake unexpectedly connected");

    assert!(error.message().contains("503"), "unexpected error: {error}");
    assert!(error.is_server_error());
    assert!(error.is_retryable());
    assert_eq!(server.handshakes().len(), 1);
    server.shutdown().await;
}

#[tokio::test]
async fn quota_handshake_summary_discards_secret_headers_and_body() {
    const HEADER_SECRET: &str = "header-secret.zyxwvutsrqponmlkjihgfedcba";
    const BODY_SECRET: &str = "body-secret.abcdefghijklmnopqrstuvwxyz";
    let server = HandshakeServer::start(vec![HandshakeOutcome::Reject {
        status: StatusCode::TOO_MANY_REQUESTS,
        body: r#"{"error":{"code":1113,"message":"Authorization: Bearer body-secret.abcdefghijklmnopqrstuvwxyz"}}"#,
        retry_after: Some("1"),
        response_header: Some(("x-provider-debug", HEADER_SECRET)),
    }])
    .await;
    let config = RealtimeTransportConfig::builder()
        .connect_timeout(Duration::from_secs(2))
        .max_connect_attempts(3)
        .try_build()
        .unwrap();

    let error = client_for(&server, config)
        .session(GLM_realtime_flash {})
        .build()
        .await
        .err()
        .expect("quota handshake unexpectedly connected");

    assert!(error.is_rate_limit());
    assert!(!error.is_retryable());
    let ZaiError::RealtimeError(kind) = &error else {
        panic!("quota handshake lost its realtime error shape: {error:?}");
    };
    let RealtimeErrorKind::HandshakeHttp(context) = kind.as_ref() else {
        panic!("quota handshake was not reduced to a safe summary: {kind:?}");
    };
    assert_eq!(context.status(), 429);
    assert_eq!(context.business_code(), Some(1113));
    assert_eq!(context.retry_after(), Some(Duration::from_secs(1)));

    for rendered in [
        format!("{error:?}"),
        format!("{error:#?}"),
        format!("{error}"),
        error.message(),
        error.compact(),
    ] {
        for secret in [HEADER_SECRET, BODY_SECRET, TEST_KEY] {
            assert!(
                !rendered.contains(secret),
                "handshake error leaked a secret: {rendered}"
            );
        }
        assert!(!rendered.contains("zyxwvutsrqponmlkjihgfedcba"));
        assert!(!rendered.contains("abcdefghijklmnopqrstuvwxyz"));
    }
    assert_eq!(server.handshakes().len(), 1);
    server.shutdown().await;
}
