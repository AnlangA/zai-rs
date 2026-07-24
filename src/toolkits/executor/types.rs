//! Executor configuration, result, and construction types.

use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::time::{Duration, SystemTime};

use dashmap::DashMap;
use serde::Serialize;

use super::{RegisteredTool, ToolExecutor};
use crate::toolkits::cache::ToolCallCache;

/// Retry configuration with exponential backoff.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts after the first try.
    pub max_retries: u32,
    /// Delay before the first retry.
    pub initial_delay: Duration,
    /// Upper bound on the per-attempt delay.
    pub max_delay: Duration,
    /// Multiplier applied to the delay between successive retries.
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            // Arbitrary tools may have side effects, so retries require an
            // explicit idempotency decision by the caller.
            max_retries: 0,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryConfig {
    /// Compute the delay before the given one-based retry attempt.
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::ZERO;
        }

        // Public fields allow direct construction. A neutral multiplier keeps
        // malformed values bounded instead of turning them into long sleeps.
        let initial_ms = self.initial_delay.as_millis() as f64;
        let max_ms = self.max_delay.as_millis() as f64;
        let multiplier = if self.backoff_multiplier.is_finite() && self.backoff_multiplier >= 1.0 {
            self.backoff_multiplier
        } else {
            1.0
        };
        let raw = initial_ms * multiplier.powf(f64::from(attempt - 1));
        let capped = if raw.is_finite() { raw } else { max_ms };

        Duration::from_millis(capped.clamp(0.0, max_ms) as u64)
    }
}

/// Tool execution configuration.
#[derive(Debug, Clone)]
pub struct ExecutionConfig {
    /// Per-call execution timeout (`None` disables the local deadline).
    ///
    /// Expiry drops the local future and returns
    /// [`ToolError::TimeoutError`](crate::toolkits::error::ToolError::TimeoutError).
    /// A remote side effect may already have been submitted and cannot be
    /// cancelled by dropping that future.
    pub timeout: Option<Duration>,
    /// Retry and backoff configuration.
    pub retry_config: RetryConfig,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            timeout: Some(Duration::from_secs(30)),
            retry_config: RetryConfig::default(),
        }
    }
}

/// Detailed outcome of one tool execution.
#[derive(Debug, Clone, Serialize)]
pub struct ExecutionResult {
    /// Name of the tool that was executed.
    pub tool_name: String,
    /// Tool return value, or JSON null after failure.
    pub result: serde_json::Value,
    /// Wall-clock duration including retries.
    pub duration: Duration,
    /// Whether execution succeeded.
    pub success: bool,
    /// Sanitized error message when execution failed.
    pub error: Option<String>,
    /// Number of retries performed after the first attempt.
    pub retries: u32,
    /// System time when execution completed.
    pub timestamp: SystemTime,
    /// Whether the result came from the executor cache.
    pub cache_hit: bool,
}

impl ExecutionResult {
    pub(super) fn success(
        tool_name: String,
        result: serde_json::Value,
        duration: Duration,
        retries: u32,
    ) -> Self {
        Self {
            tool_name,
            result,
            duration,
            success: true,
            error: None,
            retries,
            timestamp: SystemTime::now(),
            cache_hit: false,
        }
    }

    pub(super) fn failure(
        tool_name: String,
        error: String,
        duration: Duration,
        retries: u32,
    ) -> Self {
        Self {
            tool_name,
            result: serde_json::Value::Null,
            duration,
            success: false,
            error: Some(error),
            retries,
            timestamp: SystemTime::now(),
            cache_hit: false,
        }
    }

    pub(super) fn with_cache_hit(mut self) -> Self {
        self.cache_hit = true;
        self
    }
}

/// Fluent builder for a [`ToolExecutor`].
pub struct ExecutorBuilder {
    pub(super) config: ExecutionConfig,
    cache_enabled: bool,
    cache_ttl: Duration,
    cache_max_size: usize,
}

impl Default for ExecutorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutorBuilder {
    /// Create a builder with a 30-second timeout and caching disabled.
    pub fn new() -> Self {
        Self {
            config: ExecutionConfig::default(),
            cache_enabled: false,
            cache_ttl: Duration::from_secs(300),
            cache_max_size: 1000,
        }
    }

    /// Set the local deadline for each tool call.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = Some(timeout);
        self
    }

    /// Set the maximum number of retries after the first attempt.
    ///
    /// This is a global upper bound. A tool is retried only when it also opts
    /// in with [`RetryPolicy::Idempotent`](crate::toolkits::core::RetryPolicy::Idempotent)
    /// and the returned failure is retryable.
    pub fn retries(mut self, retries: u32) -> Self {
        self.config.retry_config.max_retries = retries;
        self
    }

    /// Enable result caching for tools that explicitly declare cache safety.
    pub fn enable_cache(mut self) -> Self {
        self.cache_enabled = true;
        self
    }

    /// Configure cache retention without implicitly enabling caching.
    pub fn cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = ttl;
        self
    }

    /// Configure cache capacity without implicitly enabling caching.
    pub fn cache_max_size(mut self, size: usize) -> Self {
        self.cache_max_size = size;
        self
    }

    /// Build an executor with an empty registry and independent cache.
    pub fn build(self) -> ToolExecutor {
        let cache = ToolCallCache::new()
            .with_enabled(self.cache_enabled)
            .with_ttl(self.cache_ttl)
            .with_max_size(self.cache_max_size);

        ToolExecutor {
            tools: Arc::new(DashMap::<String, RegisteredTool>::new()),
            next_generation: Arc::new(AtomicU64::new(0)),
            registry_mutation_lock: Arc::new(std::sync::Mutex::new(())),
            config: self.config,
            cache,
            cache_flights: Arc::new(DashMap::new()),
            cache_global_epoch: Arc::new(AtomicU64::new(0)),
            cache_epoch_fence: Arc::new(std::sync::RwLock::new(())),
        }
    }
}
