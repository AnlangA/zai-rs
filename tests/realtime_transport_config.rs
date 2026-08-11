//! External contract coverage for the feature-gated realtime transport policy.
#![cfg(feature = "realtime")]

use std::{
    fmt::Debug,
    future::{Future, pending},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use async_trait::async_trait;
use futures_util::StreamExt as _;
use zai_rs::{
    ZaiResult,
    model::GLM_realtime_flash,
    realtime::{
        RealtimeClient, RealtimeSession, RealtimeTransport, RealtimeTransportConfig,
        RealtimeTransportConfigBuilder, TungsteniteTransport, WsMessage,
    },
};

const TEST_KEY: &str = "test.12345678901234567890";

fn fully_custom_config() -> RealtimeTransportConfig {
    let builder: RealtimeTransportConfigBuilder = RealtimeTransportConfig::builder();
    builder
        .connect_timeout(Duration::from_secs(2))
        .max_connect_attempts(1)
        .write_timeout(Duration::from_secs(4))
        .pong_timeout(Duration::from_secs(2))
        .close_timeout(Duration::from_secs(3))
        .inbound_idle_timeout(Duration::from_secs(7))
        .outbound_queue_timeout(Duration::from_millis(250))
        .outbound_queue_capacity(3)
        .writer_queue_capacity(5)
        .event_buffer_capacity(4)
        .audio_buffer_capacity(2)
        .max_frame_bytes(128 * 1024)
        .try_build()
        .expect("the public builder should accept a valid complete policy")
}

fn short_session_config() -> RealtimeTransportConfig {
    RealtimeTransportConfig::builder()
        .connect_timeout(Duration::from_secs(1))
        .write_timeout(Duration::from_secs(1))
        .pong_timeout(Duration::from_secs(1))
        .close_timeout(Duration::from_secs(1))
        .inbound_idle_timeout(Duration::from_secs(3))
        .outbound_queue_timeout(Duration::ZERO)
        .outbound_queue_capacity(1)
        .writer_queue_capacity(1)
        .event_buffer_capacity(1)
        .audio_buffer_capacity(1)
        .max_frame_bytes(RealtimeTransportConfig::MIN_FRAME_BYTES)
        .try_build()
        .expect("short non-default session policy should be valid")
}

#[test]
fn default_and_builder_expose_every_public_setting() {
    fn assert_clone_and_debug<T: Clone + Debug>() {}

    assert_clone_and_debug::<RealtimeTransportConfig>();

    let defaults = RealtimeTransportConfig::default();
    assert_eq!(defaults.connect_timeout(), Duration::from_secs(10));
    assert_eq!(defaults.max_connect_attempts(), 3);
    assert_eq!(defaults.write_timeout(), Duration::from_secs(30));
    assert_eq!(defaults.pong_timeout(), Duration::from_secs(10));
    assert_eq!(defaults.close_timeout(), Duration::from_secs(5));
    assert_eq!(defaults.inbound_idle_timeout(), Duration::from_secs(90));
    assert_eq!(defaults.outbound_queue_timeout(), Duration::from_secs(30));
    assert_eq!(defaults.outbound_queue_capacity(), 8);
    assert_eq!(defaults.writer_queue_capacity(), 8);
    assert_eq!(defaults.event_buffer_capacity(), 8);
    assert_eq!(defaults.audio_buffer_capacity(), 8);
    assert_eq!(defaults.max_frame_bytes(), 2 * 1024 * 1024);
    assert_eq!(
        RealtimeTransportConfigBuilder::new().try_build().unwrap(),
        defaults
    );

    let custom = fully_custom_config();
    // Clone explicitly: Copy is intentionally not part of this public contract.
    let cloned = custom.clone();
    assert_eq!(custom, cloned);
    assert!(format!("{custom:?}").contains("RealtimeTransportConfig"));
    assert_eq!(custom.connect_timeout(), Duration::from_secs(2));
    assert_eq!(custom.max_connect_attempts(), 1);
    assert_eq!(custom.write_timeout(), Duration::from_secs(4));
    assert_eq!(custom.pong_timeout(), Duration::from_secs(2));
    assert_eq!(custom.close_timeout(), Duration::from_secs(3));
    assert_eq!(custom.inbound_idle_timeout(), Duration::from_secs(7));
    assert_eq!(custom.outbound_queue_timeout(), Duration::from_millis(250));
    assert_eq!(custom.outbound_queue_capacity(), 3);
    assert_eq!(custom.writer_queue_capacity(), 5);
    assert_eq!(custom.event_buffer_capacity(), 4);
    assert_eq!(custom.audio_buffer_capacity(), 2);
    assert_eq!(custom.max_frame_bytes(), 128 * 1024);
}

#[test]
fn connect_attempt_boundaries_are_public_and_checked() {
    let one = RealtimeTransportConfig::builder()
        .max_connect_attempts(1)
        .try_build()
        .expect("one connect attempt should disable retries");
    let three = RealtimeTransportConfig::builder()
        .max_connect_attempts(RealtimeTransportConfig::MAX_CONNECT_ATTEMPTS)
        .try_build()
        .expect("the documented connect-attempt maximum should be valid");

    assert_eq!(one.max_connect_attempts(), 1);
    assert_eq!(three.max_connect_attempts(), 3);
    assert_eq!(
        RealtimeTransportConfig::DEFAULT_MAX_CONNECT_ATTEMPTS,
        three.max_connect_attempts()
    );
    assert!(
        RealtimeTransportConfig::builder()
            .max_connect_attempts(0)
            .try_build()
            .is_err()
    );
    assert!(
        RealtimeTransportConfig::builder()
            .max_connect_attempts(RealtimeTransportConfig::MAX_CONNECT_ATTEMPTS + 1)
            .try_build()
            .is_err()
    );
}

#[test]
fn client_sessions_snapshot_policy_and_builder_override_wins() {
    let inherited = fully_custom_config();
    let override_config = short_session_config();
    let client = RealtimeClient::new(TEST_KEY).with_transport_config(inherited.clone());

    let snapshotted = client.session(GLM_realtime_flash {});
    assert_eq!(client.transport_config(), &inherited);
    assert_eq!(snapshotted.transport_config(), &inherited);

    let changed_client = client.with_transport_config(override_config.clone());
    assert_eq!(changed_client.transport_config(), &override_config);
    assert_eq!(snapshotted.transport_config(), &inherited);

    let overridden = snapshotted.with_transport_config(override_config.clone());
    assert_eq!(overridden.transport_config(), &override_config);
    overridden
        .validate()
        .expect("the builder override should pass zero-network validation");
}

fn assert_session_future<F>(_future: F)
where
    F: Future<Output = ZaiResult<RealtimeSession>>,
{
}

fn assert_transport_future<F>(_future: F)
where
    F: Future<Output = ZaiResult<TungsteniteTransport>>,
{
}

#[test]
fn legacy_and_configured_network_entry_points_type_check_without_polling() {
    let client = RealtimeClient::new(TEST_KEY);
    assert_session_future(client.session(GLM_realtime_flash {}).build());

    assert_transport_future(TungsteniteTransport::connect(
        "wss://example.invalid/realtime",
        "Bearer test-token",
    ));
    assert_transport_future(TungsteniteTransport::connect_with_config(
        "wss://example.invalid/realtime",
        "Bearer test-token",
        fully_custom_config(),
    ));
}

struct ThreeMethodTransport {
    sends: Arc<AtomicUsize>,
    closes: Arc<AtomicUsize>,
}

#[async_trait]
impl RealtimeTransport for ThreeMethodTransport {
    async fn send(&mut self, _msg: String) -> ZaiResult<()> {
        self.sends.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn recv(&mut self) -> ZaiResult<Option<WsMessage>> {
        pending().await
    }

    async fn close(&mut self) -> ZaiResult<()> {
        self.closes.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[tokio::test(start_paused = true)]
async fn three_method_transport_uses_the_effective_session_policy() {
    let config = short_session_config();
    let sends = Arc::new(AtomicUsize::new(0));
    let closes = Arc::new(AtomicUsize::new(0));
    let transport = ThreeMethodTransport {
        sends: Arc::clone(&sends),
        closes: Arc::clone(&closes),
    };

    let session = RealtimeClient::new(TEST_KEY)
        .session(GLM_realtime_flash {})
        .with_transport_config(config.clone())
        .build_with_transport(transport)
        .await
        .expect("the default send_confirmed method should initialize the session");

    assert_eq!(sends.load(Ordering::SeqCst), 1);
    assert_eq!(session.transport_config(), &config);

    let mut events = session.events();
    tokio::task::yield_now().await;
    tokio::time::advance(config.inbound_idle_timeout()).await;
    let error = events
        .next()
        .await
        .expect("the stream should report the configured idle timeout")
        .expect_err("the configured idle deadline unexpectedly succeeded");
    assert!(error.message().contains("inbound heartbeat"));
    assert_eq!(closes.load(Ordering::SeqCst), 1);
    drop(events);

    let close_error = session
        .close()
        .await
        .expect_err("close should preserve the background idle-timeout failure");
    assert!(close_error.message().contains("inbound heartbeat"));
}
