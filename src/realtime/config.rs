//! Validated transport and session policy for the realtime API.

use std::time::Duration;

use crate::{
    ZaiResult,
    client::{error::codes::SDK_CONFIG, transport::limits::WS_FRAME_MAX},
};

/// Transport and bounded-session policy for realtime connections.
///
/// The primary timeout and capacity defaults retain the SDK's prior fixed
/// values where applicable, while adding a bounded 30-second outbound
/// admission policy and a shorter data-frame stall guard. Sessions built by
/// [`RealtimeClient`](super::RealtimeClient) with the built-in Tungstenite
/// path use every setting. A directly constructed
/// [`TungsteniteTransport`](super::TungsteniteTransport) performs one connection
/// attempt and consumes the remaining wire-side settings; it retains the
/// complete policy, but does not consume the session-level connection-attempt
/// limit or own session queues and broadcast buffers. An already-connected
/// transport supplied through
/// [`SessionBuilder::build_with_transport`](super::SessionBuilder::build_with_transport)
/// uses only the session-owned queue, buffer, idle, send, initial-update, and
/// close policies. Connect, Pong, frame, and writer settings cannot be imposed
/// on an injected transport.
///
/// | Setting | Default | Accepted range |
/// |---|---:|---:|
/// | built-in session connect budget / direct connect deadline | 10 s | `> 0` to 60 s |
/// | built-in session connect attempts | 3 | 1 to 3 |
/// | write timeout | 30 s | 1 s to 5 min |
/// | Pong timeout | 10 s | 1 s to 60 s, no greater than write timeout |
/// | socket close timeout | 5 s | `> 0` to 60 s |
/// | inbound idle timeout | 90 s | `> write + 1 s` to 24 h |
/// | outbound admission timeout | 30 s | 0 (fail fast) to 5 min |
/// | outbound/writer queue capacity | 8 each | 1 to 64 each |
/// | event/audio buffer capacity | 8 each | 1, 2, 4, or 8 |
/// | maximum WebSocket frame | 2 MiB | 64 KiB to 2 MiB |
///
/// The 8 MiB message/end-to-end session byte budget, 8 MiB direct-transport
/// writer budget, 4 MiB raw media limit, and single concurrent outbound
/// preparation remain non-configurable safety ceilings. For a built-in
/// session, message-count admission is also end-to-end and uses the smaller of
/// the outbound and writer queue capacities; the accepted command keeps both
/// its byte and count permits until the socket writer finishes it. Fields are
/// private so future policy extensions remain source-compatible; use
/// [`RealtimeTransportConfig::builder`] to customize a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealtimeTransportConfig {
    connect_timeout: Duration,
    max_connect_attempts: u8,
    write_timeout: Duration,
    pong_timeout: Duration,
    close_timeout: Duration,
    inbound_idle_timeout: Duration,
    outbound_queue_timeout: Duration,
    outbound_queue_capacity: usize,
    writer_queue_capacity: usize,
    event_buffer_capacity: usize,
    audio_buffer_capacity: usize,
    max_frame_bytes: usize,
}

impl RealtimeTransportConfig {
    /// Default absolute budget for built-in session connection acquisition and
    /// default single-attempt deadline for a direct transport connection.
    pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
    /// Maximum absolute budget for built-in session connection acquisition and
    /// maximum single-attempt deadline for a direct transport connection.
    pub const MAX_CONNECT_TIMEOUT: Duration = Duration::from_secs(60);
    /// Default maximum number of built-in session connection attempts.
    pub const DEFAULT_MAX_CONNECT_ATTEMPTS: u8 = 3;
    /// Hard maximum number of built-in session connection attempts.
    pub const MAX_CONNECT_ATTEMPTS: u8 = 3;
    /// Default complete-message write deadline.
    pub const DEFAULT_WRITE_TIMEOUT: Duration = Duration::from_secs(30);
    /// Minimum complete-message write deadline.
    pub const MIN_WRITE_TIMEOUT: Duration = Duration::from_secs(1);
    /// Maximum complete-message write deadline.
    pub const MAX_WRITE_TIMEOUT: Duration = Duration::from_secs(5 * 60);
    /// Default Pong deadline.
    pub const DEFAULT_PONG_TIMEOUT: Duration = Duration::from_secs(10);
    /// Minimum Pong deadline.
    pub const MIN_PONG_TIMEOUT: Duration = Duration::from_secs(1);
    /// Maximum Pong deadline.
    pub const MAX_PONG_TIMEOUT: Duration = Duration::from_secs(60);
    /// Default graceful socket-close deadline.
    pub const DEFAULT_CLOSE_TIMEOUT: Duration = Duration::from_secs(5);
    /// Maximum graceful socket-close deadline.
    pub const MAX_CLOSE_TIMEOUT: Duration = Duration::from_secs(60);
    /// Default inbound application-idle deadline.
    pub const DEFAULT_INBOUND_IDLE_TIMEOUT: Duration = Duration::from_secs(90);
    /// Maximum inbound application-idle deadline.
    pub const MAX_INBOUND_IDLE_TIMEOUT: Duration = Duration::from_secs(24 * 60 * 60);
    /// Default total outbound admission deadline.
    pub const DEFAULT_OUTBOUND_QUEUE_TIMEOUT: Duration = Duration::from_secs(30);
    /// Maximum total outbound admission deadline.
    pub const MAX_OUTBOUND_QUEUE_TIMEOUT: Duration = Duration::from_secs(5 * 60);
    /// Default capacity for each configurable realtime queue/buffer.
    pub const DEFAULT_QUEUE_CAPACITY: usize = 8;
    /// Maximum capacity of the outbound and built-in writer queues.
    pub const MAX_QUEUE_CAPACITY: usize = 64;
    /// Maximum capacity of decoded event and audio broadcast buffers.
    pub const MAX_BROADCAST_CAPACITY: usize = 8;
    /// Default maximum WebSocket frame size (2 MiB).
    pub const DEFAULT_MAX_FRAME_BYTES: usize = WS_FRAME_MAX as usize;
    /// Minimum configurable WebSocket frame size (64 KiB).
    pub const MIN_FRAME_BYTES: usize = 64 * 1024;
    /// Hard maximum configurable WebSocket frame size (2 MiB).
    pub const MAX_FRAME_BYTES: usize = WS_FRAME_MAX as usize;

    /// Start a checked policy builder initialized with the defaults.
    pub fn builder() -> RealtimeTransportConfigBuilder {
        RealtimeTransportConfigBuilder::default()
    }

    /// Validate all individual and cross-field invariants.
    pub fn validate(&self) -> ZaiResult<()> {
        validate_nonzero_max(
            "connect_timeout",
            self.connect_timeout,
            Self::MAX_CONNECT_TIMEOUT,
        )?;
        if !(1..=Self::MAX_CONNECT_ATTEMPTS).contains(&self.max_connect_attempts) {
            return Err(config_error(format!(
                "max_connect_attempts must be in 1..={}",
                Self::MAX_CONNECT_ATTEMPTS
            )));
        }
        validate_range(
            "write_timeout",
            self.write_timeout,
            Self::MIN_WRITE_TIMEOUT,
            Self::MAX_WRITE_TIMEOUT,
        )?;
        validate_range(
            "pong_timeout",
            self.pong_timeout,
            Self::MIN_PONG_TIMEOUT,
            Self::MAX_PONG_TIMEOUT,
        )?;
        if self.pong_timeout > self.write_timeout {
            return Err(config_error("pong_timeout must not exceed write_timeout"));
        }
        // Check every derived deadline before calling its infallible accessor.
        // This keeps validation fallible even if public ranges are widened in
        // a future release.
        let transport_send_timeout = self
            .write_timeout
            .checked_add(Duration::from_secs(1))
            .ok_or_else(|| config_error("write_timeout derived deadline overflow"))?;
        self.write_timeout
            .checked_add(Duration::from_secs(2))
            .ok_or_else(|| config_error("write_timeout derived deadline overflow"))?;
        validate_nonzero_max("close_timeout", self.close_timeout, Self::MAX_CLOSE_TIMEOUT)?;
        self.close_timeout
            .checked_add(Duration::from_secs(2))
            .ok_or_else(|| config_error("close_timeout derived deadline overflow"))?;
        validate_nonzero_max(
            "inbound_idle_timeout",
            self.inbound_idle_timeout,
            Self::MAX_INBOUND_IDLE_TIMEOUT,
        )?;
        if self.inbound_idle_timeout <= transport_send_timeout {
            return Err(config_error(
                "inbound_idle_timeout must be greater than write_timeout + 1s",
            ));
        }
        if self.outbound_queue_timeout > Self::MAX_OUTBOUND_QUEUE_TIMEOUT {
            return Err(config_error(format!(
                "outbound_queue_timeout must be in 0s..={:?}",
                Self::MAX_OUTBOUND_QUEUE_TIMEOUT
            )));
        }
        validate_queue_capacity("outbound_queue_capacity", self.outbound_queue_capacity)?;
        validate_queue_capacity("writer_queue_capacity", self.writer_queue_capacity)?;
        validate_broadcast_capacity("event_buffer_capacity", self.event_buffer_capacity)?;
        validate_broadcast_capacity("audio_buffer_capacity", self.audio_buffer_capacity)?;
        if !(Self::MIN_FRAME_BYTES..=Self::MAX_FRAME_BYTES).contains(&self.max_frame_bytes) {
            return Err(config_error(format!(
                "max_frame_bytes must be in {}..={}",
                Self::MIN_FRAME_BYTES,
                Self::MAX_FRAME_BYTES
            )));
        }
        Ok(())
    }

    /// Absolute built-in session connection-acquisition budget.
    ///
    /// All connection attempts, retry backoff, and honored `Retry-After` delays
    /// share this one budget. A direct
    /// [`TungsteniteTransport::connect_with_config`](super::TungsteniteTransport::connect_with_config)
    /// call performs one attempt and uses the same value as that attempt's
    /// deadline.
    pub const fn connect_timeout(&self) -> Duration {
        self.connect_timeout
    }

    /// Maximum connection attempts for a built-in [`SessionBuilder`](super::SessionBuilder),
    /// inclusive of the first attempt.
    ///
    /// One disables connection retries. Directly calling
    /// [`TungsteniteTransport::connect_with_config`](super::TungsteniteTransport::connect_with_config)
    /// remains a single-attempt operation and only retains this value for policy
    /// inspection; injected transports are already connected and do not use it.
    pub const fn max_connect_attempts(&self) -> u8 {
        self.max_connect_attempts
    }

    /// Complete-message write deadline.
    pub const fn write_timeout(&self) -> Duration {
        self.write_timeout
    }

    /// Pong-response deadline (built-in transport only).
    pub const fn pong_timeout(&self) -> Duration {
        self.pong_timeout
    }

    /// Graceful socket-close deadline for the built-in transport.
    ///
    /// The session-level guard used for injected transports is derived as this
    /// value plus two seconds.
    pub const fn close_timeout(&self) -> Duration {
        self.close_timeout
    }

    /// Maximum interval without an inbound application text event.
    pub const fn inbound_idle_timeout(&self) -> Duration {
        self.inbound_idle_timeout
    }

    /// Total outbound admission deadline; zero means fail fast.
    pub const fn outbound_queue_timeout(&self) -> Duration {
        self.outbound_queue_timeout
    }

    /// Session outbound command queue capacity.
    ///
    /// A built-in session's end-to-end message-count bound is the smaller of
    /// this value and [`Self::writer_queue_capacity`].
    pub const fn outbound_queue_capacity(&self) -> usize {
        self.outbound_queue_capacity
    }

    /// Built-in socket writer queue capacity.
    ///
    /// A built-in session's end-to-end message-count bound is the smaller of
    /// this value and [`Self::outbound_queue_capacity`]. A direct
    /// `TungsteniteTransport` applies this value only to its writer queue.
    pub const fn writer_queue_capacity(&self) -> usize {
        self.writer_queue_capacity
    }

    /// Decoded server-event broadcast capacity.
    pub const fn event_buffer_capacity(&self) -> usize {
        self.event_buffer_capacity
    }

    /// Decoded audio broadcast capacity.
    pub const fn audio_buffer_capacity(&self) -> usize {
        self.audio_buffer_capacity
    }

    /// Maximum WebSocket frame size in bytes (built-in transport only).
    pub const fn max_frame_bytes(&self) -> usize {
        self.max_frame_bytes
    }

    pub(crate) fn confirmed_write_timeout(&self) -> Duration {
        self.write_timeout
            .checked_add(Duration::from_secs(1))
            .expect("validated realtime write timeout cannot overflow")
    }

    pub(crate) fn transport_send_timeout(&self) -> Duration {
        self.confirmed_write_timeout()
    }

    pub(crate) fn initial_update_timeout(&self) -> Duration {
        self.write_timeout
            .checked_add(Duration::from_secs(2))
            .expect("validated realtime write timeout cannot overflow")
    }

    pub(crate) fn writer_join_timeout(&self) -> Duration {
        self.close_timeout
            .checked_add(Duration::from_secs(1))
            .expect("validated realtime close timeout cannot overflow")
    }

    pub(crate) fn transport_close_timeout(&self) -> Duration {
        self.close_timeout
            .checked_add(Duration::from_secs(2))
            .expect("validated realtime close timeout cannot overflow")
    }

    pub(crate) fn data_frame_stall_timeout(&self) -> Duration {
        Duration::from_secs(5).min(self.pong_timeout / 2)
    }
}

impl Default for RealtimeTransportConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Self::DEFAULT_CONNECT_TIMEOUT,
            max_connect_attempts: Self::DEFAULT_MAX_CONNECT_ATTEMPTS,
            write_timeout: Self::DEFAULT_WRITE_TIMEOUT,
            pong_timeout: Self::DEFAULT_PONG_TIMEOUT,
            close_timeout: Self::DEFAULT_CLOSE_TIMEOUT,
            inbound_idle_timeout: Self::DEFAULT_INBOUND_IDLE_TIMEOUT,
            outbound_queue_timeout: Self::DEFAULT_OUTBOUND_QUEUE_TIMEOUT,
            outbound_queue_capacity: Self::DEFAULT_QUEUE_CAPACITY,
            writer_queue_capacity: Self::DEFAULT_QUEUE_CAPACITY,
            event_buffer_capacity: Self::DEFAULT_QUEUE_CAPACITY,
            audio_buffer_capacity: Self::DEFAULT_QUEUE_CAPACITY,
            max_frame_bytes: Self::DEFAULT_MAX_FRAME_BYTES,
        }
    }
}

/// Order-independent builder for [`RealtimeTransportConfig`].
///
/// Setters only record values; [`Self::try_build`] performs all individual and
/// cross-field checks so changing setter order cannot change validity.
#[derive(Debug, Clone, Default)]
pub struct RealtimeTransportConfigBuilder {
    config: RealtimeTransportConfig,
}

impl RealtimeTransportConfigBuilder {
    /// Create a builder initialized with the default policy.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the absolute built-in session connection-acquisition budget.
    ///
    /// All attempts, retry backoff, and honored `Retry-After` delays share this
    /// budget. A direct transport connection instead uses it as its single-
    /// attempt deadline.
    pub fn connect_timeout(mut self, value: Duration) -> Self {
        self.config.connect_timeout = value;
        self
    }

    /// Set the built-in session connection-attempt limit, inclusive of the
    /// first attempt. One disables connection retries.
    pub fn max_connect_attempts(mut self, value: u8) -> Self {
        self.config.max_connect_attempts = value;
        self
    }

    /// Set the complete-message write deadline.
    pub fn write_timeout(mut self, value: Duration) -> Self {
        self.config.write_timeout = value;
        self
    }

    /// Set the Pong-response deadline.
    pub fn pong_timeout(mut self, value: Duration) -> Self {
        self.config.pong_timeout = value;
        self
    }

    /// Set the graceful socket-close deadline.
    pub fn close_timeout(mut self, value: Duration) -> Self {
        self.config.close_timeout = value;
        self
    }

    /// Set the inbound application-idle deadline.
    pub fn inbound_idle_timeout(mut self, value: Duration) -> Self {
        self.config.inbound_idle_timeout = value;
        self
    }

    /// Set the total outbound admission deadline; zero enables fail-fast mode.
    pub fn outbound_queue_timeout(mut self, value: Duration) -> Self {
        self.config.outbound_queue_timeout = value;
        self
    }

    /// Set the session outbound queue capacity.
    ///
    /// Built-in sessions use the smaller outbound/writer capacity across the
    /// complete admission-to-write pipeline.
    pub fn outbound_queue_capacity(mut self, value: usize) -> Self {
        self.config.outbound_queue_capacity = value;
        self
    }

    /// Set the built-in writer queue capacity.
    ///
    /// Built-in sessions use the smaller outbound/writer capacity across the
    /// complete admission-to-write pipeline.
    pub fn writer_queue_capacity(mut self, value: usize) -> Self {
        self.config.writer_queue_capacity = value;
        self
    }

    /// Set the decoded event broadcast capacity (`1`, `2`, `4`, or `8`).
    pub fn event_buffer_capacity(mut self, value: usize) -> Self {
        self.config.event_buffer_capacity = value;
        self
    }

    /// Set the decoded audio broadcast capacity (`1`, `2`, `4`, or `8`).
    pub fn audio_buffer_capacity(mut self, value: usize) -> Self {
        self.config.audio_buffer_capacity = value;
        self
    }

    /// Set the maximum WebSocket frame size in bytes.
    pub fn max_frame_bytes(mut self, value: usize) -> Self {
        self.config.max_frame_bytes = value;
        self
    }

    /// Validate and build the policy.
    pub fn try_build(self) -> ZaiResult<RealtimeTransportConfig> {
        self.config.validate()?;
        Ok(self.config)
    }
}

fn validate_nonzero_max(name: &str, value: Duration, max: Duration) -> ZaiResult<()> {
    if value.is_zero() || value > max {
        return Err(config_error(format!(
            "{name} must be greater than zero and no greater than {max:?}"
        )));
    }
    Ok(())
}

fn validate_range(name: &str, value: Duration, min: Duration, max: Duration) -> ZaiResult<()> {
    if value < min || value > max {
        return Err(config_error(format!("{name} must be in {min:?}..={max:?}")));
    }
    Ok(())
}

fn validate_queue_capacity(name: &str, value: usize) -> ZaiResult<()> {
    if !(1..=RealtimeTransportConfig::MAX_QUEUE_CAPACITY).contains(&value) {
        return Err(config_error(format!(
            "{name} must be in 1..={}",
            RealtimeTransportConfig::MAX_QUEUE_CAPACITY
        )));
    }
    Ok(())
}

fn validate_broadcast_capacity(name: &str, value: usize) -> ZaiResult<()> {
    if !value.is_power_of_two() || value > RealtimeTransportConfig::MAX_BROADCAST_CAPACITY {
        return Err(config_error(format!(
            "{name} must be a power of two in 1..={}",
            RealtimeTransportConfig::MAX_BROADCAST_CAPACITY
        )));
    }
    Ok(())
}

fn config_error(message: impl Into<String>) -> crate::ZaiError {
    crate::ZaiError::ApiError {
        code: SDK_CONFIG,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_use_the_documented_bounded_policy() {
        let config = RealtimeTransportConfig::default();
        config.validate().unwrap();
        assert_eq!(config.connect_timeout(), Duration::from_secs(10));
        assert_eq!(config.max_connect_attempts(), 3);
        assert_eq!(config.write_timeout(), Duration::from_secs(30));
        assert_eq!(config.pong_timeout(), Duration::from_secs(10));
        assert_eq!(config.close_timeout(), Duration::from_secs(5));
        assert_eq!(config.inbound_idle_timeout(), Duration::from_secs(90));
        assert_eq!(config.outbound_queue_timeout(), Duration::from_secs(30));
        assert_eq!(config.confirmed_write_timeout(), Duration::from_secs(31));
        assert_eq!(config.initial_update_timeout(), Duration::from_secs(32));
        assert_eq!(config.writer_join_timeout(), Duration::from_secs(6));
        assert_eq!(config.transport_close_timeout(), Duration::from_secs(7));
        assert_eq!(config.max_frame_bytes(), 2 * 1024 * 1024);
    }

    #[test]
    fn builder_is_order_independent_and_checks_cross_field_rules() {
        let config = RealtimeTransportConfig::builder()
            .inbound_idle_timeout(Duration::from_secs(42))
            .max_connect_attempts(1)
            .pong_timeout(Duration::from_secs(4))
            .write_timeout(Duration::from_secs(40))
            .outbound_queue_timeout(Duration::ZERO)
            .outbound_queue_capacity(2)
            .writer_queue_capacity(3)
            .event_buffer_capacity(4)
            .audio_buffer_capacity(2)
            .max_frame_bytes(64 * 1024)
            .try_build()
            .unwrap();
        assert_eq!(config.transport_send_timeout(), Duration::from_secs(41));
        assert_eq!(config.max_connect_attempts(), 1);
        assert_eq!(config.outbound_queue_timeout(), Duration::ZERO);
        assert_eq!(config.max_frame_bytes(), 64 * 1024);

        assert!(
            RealtimeTransportConfig::builder()
                .write_timeout(Duration::from_secs(30))
                .pong_timeout(Duration::from_secs(31))
                .try_build()
                .is_err()
        );
        assert!(
            RealtimeTransportConfig::builder()
                .write_timeout(Duration::from_secs(30))
                .inbound_idle_timeout(Duration::from_secs(31))
                .try_build()
                .is_err()
        );
    }

    #[test]
    fn every_public_boundary_is_checked() {
        let minimum = RealtimeTransportConfig::builder()
            .connect_timeout(Duration::from_nanos(1))
            .max_connect_attempts(1)
            .write_timeout(RealtimeTransportConfig::MIN_WRITE_TIMEOUT)
            .pong_timeout(RealtimeTransportConfig::MIN_PONG_TIMEOUT)
            .close_timeout(Duration::from_nanos(1))
            .inbound_idle_timeout(Duration::from_secs(2) + Duration::from_nanos(1))
            .outbound_queue_timeout(Duration::ZERO)
            .outbound_queue_capacity(1)
            .writer_queue_capacity(1)
            .event_buffer_capacity(1)
            .audio_buffer_capacity(1)
            .max_frame_bytes(RealtimeTransportConfig::MIN_FRAME_BYTES)
            .try_build();
        assert!(minimum.is_ok());

        let maximum = RealtimeTransportConfig::builder()
            .connect_timeout(RealtimeTransportConfig::MAX_CONNECT_TIMEOUT)
            .max_connect_attempts(RealtimeTransportConfig::MAX_CONNECT_ATTEMPTS)
            .write_timeout(RealtimeTransportConfig::MAX_WRITE_TIMEOUT)
            .pong_timeout(RealtimeTransportConfig::MAX_PONG_TIMEOUT)
            .close_timeout(RealtimeTransportConfig::MAX_CLOSE_TIMEOUT)
            .inbound_idle_timeout(RealtimeTransportConfig::MAX_INBOUND_IDLE_TIMEOUT)
            .outbound_queue_timeout(RealtimeTransportConfig::MAX_OUTBOUND_QUEUE_TIMEOUT)
            .outbound_queue_capacity(RealtimeTransportConfig::MAX_QUEUE_CAPACITY)
            .writer_queue_capacity(RealtimeTransportConfig::MAX_QUEUE_CAPACITY)
            .event_buffer_capacity(RealtimeTransportConfig::MAX_BROADCAST_CAPACITY)
            .audio_buffer_capacity(RealtimeTransportConfig::MAX_BROADCAST_CAPACITY)
            .max_frame_bytes(RealtimeTransportConfig::MAX_FRAME_BYTES)
            .try_build();
        assert!(maximum.is_ok());

        let invalid = [
            RealtimeTransportConfig::builder()
                .connect_timeout(Duration::ZERO)
                .try_build(),
            RealtimeTransportConfig::builder()
                .connect_timeout(
                    RealtimeTransportConfig::MAX_CONNECT_TIMEOUT + Duration::from_nanos(1),
                )
                .try_build(),
            RealtimeTransportConfig::builder()
                .max_connect_attempts(0)
                .try_build(),
            RealtimeTransportConfig::builder()
                .max_connect_attempts(RealtimeTransportConfig::MAX_CONNECT_ATTEMPTS + 1)
                .try_build(),
            RealtimeTransportConfig::builder()
                .write_timeout(Duration::from_millis(999))
                .try_build(),
            RealtimeTransportConfig::builder()
                .write_timeout(RealtimeTransportConfig::MAX_WRITE_TIMEOUT + Duration::from_nanos(1))
                .inbound_idle_timeout(RealtimeTransportConfig::MAX_INBOUND_IDLE_TIMEOUT)
                .try_build(),
            RealtimeTransportConfig::builder()
                .pong_timeout(Duration::from_millis(999))
                .try_build(),
            RealtimeTransportConfig::builder()
                .pong_timeout(RealtimeTransportConfig::MAX_PONG_TIMEOUT + Duration::from_nanos(1))
                .try_build(),
            RealtimeTransportConfig::builder()
                .close_timeout(Duration::ZERO)
                .try_build(),
            RealtimeTransportConfig::builder()
                .close_timeout(RealtimeTransportConfig::MAX_CLOSE_TIMEOUT + Duration::from_nanos(1))
                .try_build(),
            RealtimeTransportConfig::builder()
                .inbound_idle_timeout(Duration::from_secs(31))
                .try_build(),
            RealtimeTransportConfig::builder()
                .inbound_idle_timeout(
                    RealtimeTransportConfig::MAX_INBOUND_IDLE_TIMEOUT + Duration::from_nanos(1),
                )
                .try_build(),
            RealtimeTransportConfig::builder()
                .outbound_queue_timeout(
                    RealtimeTransportConfig::MAX_OUTBOUND_QUEUE_TIMEOUT + Duration::from_nanos(1),
                )
                .try_build(),
            RealtimeTransportConfig::builder()
                .outbound_queue_capacity(0)
                .try_build(),
            RealtimeTransportConfig::builder()
                .writer_queue_capacity(RealtimeTransportConfig::MAX_QUEUE_CAPACITY + 1)
                .try_build(),
            RealtimeTransportConfig::builder()
                .event_buffer_capacity(3)
                .try_build(),
            RealtimeTransportConfig::builder()
                .audio_buffer_capacity(0)
                .try_build(),
            RealtimeTransportConfig::builder()
                .max_frame_bytes(RealtimeTransportConfig::MIN_FRAME_BYTES - 1)
                .try_build(),
            RealtimeTransportConfig::builder()
                .max_frame_bytes(RealtimeTransportConfig::MAX_FRAME_BYTES + 1)
                .try_build(),
        ];
        for result in invalid {
            match result.expect_err("invalid realtime boundary was accepted") {
                crate::ZaiError::ApiError { code, .. } => assert_eq!(code, SDK_CONFIG),
                error => panic!("unexpected config error: {error:?}"),
            }
        }
    }
}
