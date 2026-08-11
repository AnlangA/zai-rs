//! Public transport-injection contract for realtime sessions.
#![cfg(feature = "realtime")]

use std::{
    future::pending,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::{mpsc, oneshot};
use zai_rs::{
    ZaiError, ZaiResult,
    model::GLM_realtime_flash,
    realtime::{RealtimeClient, RealtimeTransport, TungsteniteTransport, WsMessage},
};

const TEST_KEY: &str = "test.12345678901234567890";
const TEST_TIMEOUT: Duration = Duration::from_secs(2);

#[test]
fn built_in_transport_exposes_confirmed_trait_method() {
    async fn call(transport: &mut TungsteniteTransport) -> ZaiResult<()> {
        transport.send_confirmed("{}".to_owned()).await
    }

    let _ = call;
}

struct ConfirmedRecordingTransport {
    frames: mpsc::UnboundedSender<String>,
    confirmation_started: Option<oneshot::Sender<()>>,
    release_confirmation: Option<oneshot::Receiver<()>>,
    close_count: Arc<AtomicUsize>,
}

#[async_trait]
impl RealtimeTransport for ConfirmedRecordingTransport {
    async fn send(&mut self, msg: String) -> ZaiResult<()> {
        let _ = self.frames.send(msg);
        Ok(())
    }

    async fn send_confirmed(&mut self, msg: String) -> ZaiResult<()> {
        let _ = self.frames.send(msg);
        if let Some(started) = self.confirmation_started.take() {
            let _ = started.send(());
        }
        match self.release_confirmation.take() {
            Some(release) => release.await.map_err(|_| ZaiError::ApiError {
                code: 9900,
                message: "test confirmation barrier was dropped".to_string(),
            }),
            None => Err(ZaiError::ApiError {
                code: 9900,
                message: "test confirmation barrier was reused".to_string(),
            }),
        }
    }

    async fn recv(&mut self) -> ZaiResult<Option<WsMessage>> {
        pending().await
    }

    async fn close(&mut self) -> ZaiResult<()> {
        self.close_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[tokio::test]
async fn injected_transport_confirms_init_then_preserves_fifo_and_closes() {
    let (frames_tx, mut frames_rx) = mpsc::unbounded_channel();
    let (started_tx, started_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();
    let close_count = Arc::new(AtomicUsize::new(0));
    let transport = ConfirmedRecordingTransport {
        frames: frames_tx,
        confirmation_started: Some(started_tx),
        release_confirmation: Some(release_rx),
        close_count: Arc::clone(&close_count),
    };

    let build = tokio::spawn(async move {
        RealtimeClient::new(TEST_KEY)
            .session(GLM_realtime_flash {})
            .build_with_transport(transport)
            .await
    });

    tokio::time::timeout(TEST_TIMEOUT, started_rx)
        .await
        .expect("initial send was never started")
        .expect("transport dropped the initial-send signal");
    let init = tokio::time::timeout(TEST_TIMEOUT, frames_rx.recv())
        .await
        .expect("initial frame was not observed")
        .expect("frame observer closed before session.update");
    assert!(
        !init.contains(TEST_KEY),
        "SDK credentials leaked into the injected transport payload"
    );
    let init: Value = serde_json::from_str(&init).expect("session.update must be valid JSON");
    assert_eq!(init["type"], "session.update");
    assert_eq!(init["session"]["model"], "glm-realtime-flash");
    assert!(
        !build.is_finished(),
        "build returned before the transport confirmed session.update"
    );

    release_tx
        .send(())
        .expect("build stopped waiting for initial-send confirmation");
    let session = tokio::time::timeout(TEST_TIMEOUT, build)
        .await
        .expect("build did not finish after confirmation")
        .expect("build task panicked")
        .expect("confirmed injected transport failed to build");

    session.send_text("first").await.unwrap();
    session.send_text("second").await.unwrap();

    let first = next_frame(&mut frames_rx).await;
    let second = next_frame(&mut frames_rx).await;
    assert_eq!(first["type"], "conversation.item.create");
    assert_eq!(first["item"]["content"][0]["text"], "first");
    assert_eq!(second["type"], "conversation.item.create");
    assert_eq!(second["item"]["content"][0]["text"], "second");

    session.close().await.unwrap();
    assert_eq!(close_count.load(Ordering::SeqCst), 1);
}

async fn next_frame(frames: &mut mpsc::UnboundedReceiver<String>) -> Value {
    let frame = tokio::time::timeout(TEST_TIMEOUT, frames.recv())
        .await
        .expect("timed out waiting for an injected transport frame")
        .expect("injected transport frame observer closed");
    serde_json::from_str(&frame).expect("injected transport frame must be valid JSON")
}

struct ThreeMethodInitErrorTransport {
    send_count: Arc<AtomicUsize>,
    close_count: Arc<AtomicUsize>,
}

#[async_trait]
impl RealtimeTransport for ThreeMethodInitErrorTransport {
    async fn send(&mut self, _msg: String) -> ZaiResult<()> {
        self.send_count.fetch_add(1, Ordering::SeqCst);
        Err(ZaiError::ApiError {
            code: 9901,
            message: "injected initial send failed".to_string(),
        })
    }

    async fn recv(&mut self) -> ZaiResult<Option<WsMessage>> {
        pending().await
    }

    async fn close(&mut self) -> ZaiResult<()> {
        self.close_count.fetch_add(1, Ordering::SeqCst);
        Err(ZaiError::ApiError {
            code: 9902,
            message: "injected close also failed".to_string(),
        })
    }
}

#[tokio::test]
async fn default_confirmed_send_preserves_init_error_and_still_closes() {
    let send_count = Arc::new(AtomicUsize::new(0));
    let close_count = Arc::new(AtomicUsize::new(0));
    let transport = ThreeMethodInitErrorTransport {
        send_count: Arc::clone(&send_count),
        close_count: Arc::clone(&close_count),
    };

    let error = RealtimeClient::new(TEST_KEY)
        .session(GLM_realtime_flash {})
        .build_with_transport(transport)
        .await
        .err()
        .expect("the injected initial-send failure must fail build");

    assert!(matches!(
        error,
        ZaiError::ApiError { code: 9901, ref message }
            if message == "injected initial send failed"
    ));
    assert_eq!(send_count.load(Ordering::SeqCst), 1);
    assert_eq!(close_count.load(Ordering::SeqCst), 1);
}
