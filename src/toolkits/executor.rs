//! Enhanced tool executor with type-safe builder pattern

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::{sync::Semaphore, task::JoinSet, time::timeout};
use tracing::warn;

use super::{
    cache::{CacheKey, ToolCallCache},
    core::ToolHandler,
};
use crate::{
    model::{
        chat_base_response::ToolCallMessage,
        chat_message_types::TextMessage,
        tools::{Function, Tools},
    },
    toolkits::{
        core::DynTool,
        error::{ToolError, ToolResult, error_context},
    },
};

/// Cap on how many tool calls run concurrently in
/// [`execute_tool_calls_parallel`](ToolExecutor::execute_tool_calls_parallel)
/// / [`execute_tool_calls_ordered`](ToolExecutor::execute_tool_calls_ordered).
///
/// Prevents a model that emits many tool calls in one turn from fanning them
/// all out at once and overwhelming downstream (e.g. MCP) services. Network
/// calls queue on the semaphore rather than spawning unbounded tasks.
const MAX_CONCURRENT_TOOL_CALLS: usize = 8;

/// Enhanced retry configuration with exponential backoff
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
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryConfig {
    /// Compute the delay to apply before the given (1-based) attempt.
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::ZERO;
        }

        // Compute in milliseconds as `f64` (honors the configurable `f64`
        // `backoff_multiplier`), cap at `max_delay`, then convert. The final
        // `as u64` saturates on overflow or NaN, and `max(0.0)` guards against
        // a negative product from a misconfigured multiplier.
        let initial_ms = self.initial_delay.as_millis() as f64;
        let max_ms = self.max_delay.as_millis() as f64;
        let raw = initial_ms * self.backoff_multiplier.powi((attempt - 1) as i32);
        let capped = if raw.is_finite() { raw } else { max_ms };
        let delay_ms = capped.clamp(0.0, max_ms) as u64;

        Duration::from_millis(delay_ms)
    }
}

/// Execution configuration with type-safe builder
#[derive(Debug, Clone)]
pub struct ExecutionConfig {
    /// Per-call execution timeout (`None` = no timeout).
    ///
    /// When the deadline elapses the tool's execution future is dropped
    /// (cancelled) and the call resolves to a [`ToolError::TimeoutError`]. The
    /// timeout cancels only the *local* future — for a tool backed by a remote
    /// service the request may already be on the wire (see
    /// [`ToolError::TimeoutError`] for the full caveat).
    pub timeout: Option<Duration>,
    /// Retry/backoff configuration.
    pub retry_config: RetryConfig,
    /// Whether to validate input parameters against the tool schema.
    ///
    /// **Reserved:** schema validation currently fires whenever the
    /// `tool-validation` Cargo feature is enabled (it lives inside
    /// `FunctionTool::execute_json`). This flag is reserved for a future
    /// per-call opt-out and does not yet suppress validation on its own.
    pub validate_parameters: bool,
    /// Whether to emit execution log events.
    ///
    /// **Reserved:** execution is instrumented with `tracing` spans whose output
    /// is governed by the subscriber's filter (e.g. `RUST_LOG`), independent of
    /// this flag. Kept for API stability; setting it has no effect today.
    pub enable_logging: bool,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            timeout: Some(Duration::from_secs(30)),
            retry_config: RetryConfig::default(),
            validate_parameters: true,
            enable_logging: false,
        }
    }
}

/// Execution result with enhanced metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Name of the tool that was executed.
    pub tool_name: String,
    /// The tool's return value (JSON).
    pub result: serde_json::Value,
    /// Wall-clock duration of the (possibly retried) execution.
    pub duration: Duration,
    /// Whether the execution succeeded.
    pub success: bool,
    /// Error message, present only on failure.
    pub error: Option<String>,
    /// Number of retries performed.
    pub retries: u32,
    /// When the execution completed.
    pub timestamp: std::time::SystemTime,
    /// Free-form metadata (e.g. `cache_hit`).
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

impl ExecutionResult {
    /// Construct a successful execution result.
    pub fn success(
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
            timestamp: std::time::SystemTime::now(),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Construct a failed execution result.
    pub fn failure(tool_name: String, error: String, duration: Duration, retries: u32) -> Self {
        Self {
            tool_name,
            result: serde_json::Value::Null,
            duration,
            success: false,
            error: Some(error),
            retries,
            timestamp: std::time::SystemTime::now(),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Attach a free-form metadata entry to this result.
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Enhanced tool executor with built-in registry and fluent API
#[derive(Clone)]
pub struct ToolExecutor {
    tools: Arc<DashMap<String, Arc<dyn DynTool>>>,
    config: ExecutionConfig,
    cache: ToolCallCache,
}

impl std::fmt::Debug for ToolExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tool_count = self.tools.len();
        let cache_enabled = self.cache.stats().total_entries > 0;
        f.debug_struct("ToolExecutor")
            .field("tool_count", &tool_count)
            .field("config", &self.config)
            .field("cache_enabled", &cache_enabled)
            .finish()
    }
}

impl Default for ToolExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolExecutor {
    /// Create a new executor with default config
    pub fn new() -> Self {
        Self {
            tools: Arc::new(DashMap::new()),
            config: ExecutionConfig::default(),
            cache: ToolCallCache::new(),
        }
    }

    /// Create an executor builder for fluent API
    pub fn builder() -> ExecutorBuilder {
        ExecutorBuilder::new()
    }

    /// Enable or disable tool call result caching
    pub fn with_cache_enabled(mut self, enabled: bool) -> Self {
        self.cache = self.cache.with_enabled(enabled);
        self
    }

    /// Set cache TTL (time-to-live)
    pub fn with_cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache = self.cache.with_ttl(ttl);
        self
    }

    /// Set maximum cache size
    pub fn with_cache_max_size(mut self, size: usize) -> Self {
        self.cache = self.cache.with_max_size(size);
        self
    }

    /// Clear the cache
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    /// Invalidate cache for a specific tool
    pub fn invalidate_cache_for_tool(&self, tool_name: &str) {
        self.cache.invalidate_tool(tool_name);
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> super::cache::CacheStats {
        self.cache.stats()
    }

    /// Chain-friendly: add a dynamic tool, returns error if already registered
    pub fn add_dyn_tool(&self, tool: Box<dyn DynTool>) -> ToolResult<&Self> {
        let name = tool.name().to_string();
        if self.tools.contains_key(&name) {
            return Err(ToolError::RegistrationError {
                message: format!("Tool '{}' is already registered", name).into(),
            });
        }
        self.tools.insert(name, Arc::from(tool));
        Ok(self)
    }

    /// Chain-friendly: try to add a dynamic tool (ignores error)
    pub fn try_add_dyn_tool(&self, tool: Box<dyn DynTool>) -> &Self {
        let name = tool.name().to_string();
        self.tools.entry(name).or_insert_with(|| Arc::from(tool));
        self
    }

    /// Unregister a tool
    pub fn unregister(&self, name: &str) -> ToolResult<()> {
        if self.tools.remove(name).is_none() {
            return Err(error_context().tool_not_found());
        }
        Ok(())
    }

    /// Get input schema for a tool
    pub fn input_schema(&self, name: &str) -> Option<serde_json::Value> {
        self.tools.get(name).map(|t| t.input_schema())
    }

    /// Check if tool exists
    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// List tool names
    pub fn tool_names(&self) -> Vec<String> {
        self.tools.iter().map(|entry| entry.key().clone()).collect()
    }

    fn get_tool(&self, name: &str) -> Option<Arc<dyn DynTool>> {
        self.tools.get(name).map(|t| Arc::clone(t.value()))
    }

    /// Execute a tool with detailed result and exponential backoff
    #[tracing::instrument(skip(self, input))]
    pub async fn execute(
        &self,
        tool_name: &str,
        input: serde_json::Value,
    ) -> ToolResult<ExecutionResult> {
        let start_time = Instant::now();
        let mut retries = 0;
        let retry_config = &self.config.retry_config;

        // Build the cache key lazily: `CacheKey::new` deep-clones and
        // re-serializes the arguments, so skip it entirely when the cache is
        // disabled (and only pay for it once when enabled).
        let cache_key = if self.cache.enabled() {
            Some(CacheKey::new(tool_name.to_string(), input.clone()))
        } else {
            None
        };
        if let Some(ref key) = cache_key
            && let Some(cached_result) = self.cache.get(key)
        {
            let duration = start_time.elapsed();
            return Ok(ExecutionResult::success(
                tool_name.to_string(),
                cached_result,
                duration,
                retries,
            )
            .with_metadata("cache_hit", serde_json::Value::Bool(true)));
        }

        loop {
            match self.execute_once(tool_name, &input).await {
                Ok(result) => {
                    let duration = start_time.elapsed();
                    // Cache the successful result (only if the cache is on).
                    if let Some(key) = cache_key {
                        self.cache.insert(key, result.clone(), None);
                    }

                    return Ok(ExecutionResult::success(
                        tool_name.to_string(),
                        result,
                        duration,
                        retries,
                    )
                    .with_metadata("cache_hit", serde_json::Value::Bool(false)));
                },
                Err(error) => {
                    // Only retry on retryable errors (timeout, transient failures)
                    if !error.is_retryable() {
                        let duration = start_time.elapsed();
                        return Ok(ExecutionResult::failure(
                            tool_name.to_string(),
                            error.to_string(),
                            duration,
                            retries,
                        ));
                    }

                    if retries >= retry_config.max_retries {
                        let duration = start_time.elapsed();
                        return Ok(ExecutionResult::failure(
                            tool_name.to_string(),
                            error.to_string(),
                            duration,
                            retries,
                        ));
                    }

                    retries += 1;

                    warn!(attempt = retries, error = %error, "Tool execution failed, retrying");

                    // Use exponential backoff
                    let delay = retry_config.calculate_delay(retries);
                    tokio::time::sleep(delay).await;
                },
            }
        }
    }

    /// Execute a tool and return only the result
    pub async fn execute_simple(
        &self,
        tool_name: &str,
        input: serde_json::Value,
    ) -> ToolResult<serde_json::Value> {
        let result = self.execute(tool_name, input).await?;
        if result.success {
            Ok(result.result)
        } else {
            Err(error_context()
                .with_tool(tool_name)
                .execution_failed(result.error.unwrap_or_else(|| "Unknown error".to_string())))
        }
    }

    /// Bulk load function specs from a directory of .json files and register
    /// them with handlers.
    ///
    /// - Each file should contain either of the following shapes:
    ///   1) {"name":..., "description":..., "parameters": {...}}
    ///   2) {"type":"function", "function": {"name":..., "description":...,
    ///      "parameters": {...}}}
    /// - `handlers` maps function `name` -> handler closure
    /// - `strict`: when true, missing handler for any spec will return error;
    ///   when false, specs without handlers are skipped
    ///
    /// Returns the list of function names successfully registered.
    pub fn add_functions_from_dir_with_registry(
        &self,
        dir: impl AsRef<std::path::Path>,
        handlers: &std::collections::HashMap<String, ToolHandler>,
        strict: bool,
    ) -> ToolResult<Vec<String>> {
        use std::fs;

        use serde_json::Value;
        let dir = dir.as_ref();
        let mut added = Vec::new();
        let read_dir = fs::read_dir(dir).map_err(|e| {
            error_context().invalid_parameters(format!(
                "Failed to read dir {}: {}",
                dir.display(),
                e
            ))
        })?;
        for entry in read_dir {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    return Err(
                        error_context().invalid_parameters(format!("Dir entry error: {}", e))
                    );
                },
            };
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let content = fs::read_to_string(&path).map_err(|e| {
                error_context().invalid_parameters(format!(
                    "Failed to read {}: {}",
                    path.display(),
                    e
                ))
            })?;
            let spec: Value = serde_json::from_str(&content).map_err(|e| {
                error_context().invalid_parameters(format!(
                    "Invalid JSON in {}: {}",
                    path.display(),
                    e
                ))
            })?;

            // Extract name/description/parameters from spec
            let (name, description, parameters) =
                crate::toolkits::core::parse_function_spec_details(&spec).map_err(|e| {
                    error_context().invalid_parameters(format!(
                        "Failed to parse spec {}: {}",
                        path.display(),
                        e
                    ))
                })?;

            let handler = match handlers.get(&name) {
                Some(h) => h.clone(),
                None => {
                    if strict {
                        return Err(error_context().invalid_parameters(format!(
                            "No handler registered for function '{}' (file {})",
                            name,
                            path.display()
                        )));
                    } else {
                        // skip silently
                        continue;
                    }
                },
            };

            // Build FunctionTool via existing builder path (will auto-complete schema
            // defaults)
            let mut builder =
                crate::toolkits::core::FunctionTool::builder(name.clone(), description);
            if let Some(p) = parameters {
                builder = builder.schema(p);
            }
            let tool = builder
                .handler(move |args| {
                    let h = handler.clone();
                    h(args)
                })
                .build()?;

            self.add_dyn_tool(Box::new(tool))?;
            added.push(name);
        }
        Ok(added)
    }

    /// Execute LLM tool_calls in parallel and return `TextMessage::tool`
    /// messages.
    ///
    /// Behavior:
    /// - Parses each ToolCallMessage's function.arguments (stringified JSON
    ///   supported)
    /// - Runs all tools concurrently using this executor
    /// - Captures errors per-call and encodes them as JSON: { "error": {
    ///   "type": "...", "message": "..." } }
    /// - Preserves tool_call `id` by emitting TextMessage::tool_with_id when
    ///   present
    ///
    /// Returns:
    /// - `Vec<TextMessage>` ready to be appended to ChatCompletion as tool
    ///   messages.
    async fn execute_single_tool_call(&self, tc: &ToolCallMessage) -> TextMessage {
        let id_opt = tc.id().map(std::string::ToString::to_string);
        let func_opt = tc.function();

        if let Some(func) = func_opt {
            let name = func.name().unwrap_or("").to_string();
            let args_str = func.arguments().unwrap_or("{}");
            let args_json: serde_json::Value = serde_json::from_str(args_str)
                .unwrap_or_else(|_| serde_json::json!({ "_raw": args_str }));

            let content_json = match self.execute_simple(&name, args_json).await {
                Ok(v) => v,
                Err(err) => serde_json::json!({
                    "error": { "type": "execution_failed", "message": err.to_string() }
                }),
            };

            let s = serde_json::to_string(&content_json).unwrap_or_else(|_| "{}".to_string());

            if let Some(id) = id_opt {
                TextMessage::tool_with_id(s, id)
            } else {
                TextMessage::tool(s)
            }
        } else {
            let s = serde_json::json!({
                "error": { "type": "missing_function", "message": "tool_call.function is missing" }
            })
            .to_string();

            if let Some(id) = id_opt {
                TextMessage::tool_with_id(s, id)
            } else {
                TextMessage::tool(s)
            }
        }
    }

    /// Execute LLM tool calls concurrently, returning one `TextMessage::tool`
    /// per call (order is NOT preserved — see
    /// [`Self::execute_tool_calls_ordered`]).
    pub async fn execute_tool_calls_parallel(&self, calls: &[ToolCallMessage]) -> Vec<TextMessage> {
        let mut set = JoinSet::new();
        let permits = Arc::new(Semaphore::new(MAX_CONCURRENT_TOOL_CALLS));

        // Clone each call so it can be moved into the `'static` spawned task.
        for tc in calls {
            let tc = tc.clone();
            let this = self.clone();
            let permits = Arc::clone(&permits);
            set.spawn(async move {
                // Bounded concurrency: a model emitting many tool calls no
                // longer fires them all simultaneously. `.ok()` holds the
                // `Option<Permit>` (gating the call) without panicking; `None`
                // only if the semaphore were closed, which never happens here.
                let _permit = permits.acquire().await.ok();
                this.execute_single_tool_call(&tc).await
            });
        }

        let mut messages = Vec::with_capacity(calls.len());
        while let Some(res) = set.join_next().await {
            match res {
                Ok(msg) => messages.push(msg),
                Err(join_err) => {
                    // A spawned task panicked or was cancelled. Surface it as a
                    // tool message so the caller (and the LLM) observes the
                    // failure rather than receiving a silently-shorter result
                    // vector whose length no longer matches the input.
                    warn!(
                        error = %join_err,
                        "tool execution task panicked or was cancelled"
                    );
                    messages.push(TextMessage::tool(
                        serde_json::json!({
                            "error": {
                                "type": "task_panic",
                                "message": "a tool execution task panicked or was cancelled"
                            }
                        })
                        .to_string(),
                    ));
                },
            }
        }
        messages
    }

    /// Execute LLM tool_calls in parallel with result ordering preserved
    ///
    /// This method guarantees that results are returned in the same order as
    /// the input calls, which is important for maintaining conversation
    /// context in LLM interactions.
    ///
    /// Behavior:
    /// - Parses each ToolCallMessage's function.arguments (stringified JSON
    ///   supported)
    /// - Runs all tools concurrently using this executor
    /// - Preserves the original order of tool calls in results
    /// - Captures errors per-call and encodes them as JSON
    /// - Preserves tool_call `id` by emitting TextMessage::tool_with_id when
    ///   present
    /// - Isolates panics/cancellation per call: a panicking handler surfaces as
    ///   an in-band `task_panic` tool message in its slot (matching
    ///   [`Self::execute_tool_calls_parallel`]) instead of unwinding out of this
    ///   call and discarding every result.
    ///
    /// Returns:
    /// - `Vec<TextMessage>` in the same order as input calls, ready for
    ///   ChatCompletion
    pub async fn execute_tool_calls_ordered(&self, calls: &[ToolCallMessage]) -> Vec<TextMessage> {
        let mut set = JoinSet::new();
        let permits = Arc::new(Semaphore::new(MAX_CONCURRENT_TOOL_CALLS));

        // Spawn each call carrying its original index, so results can be placed
        // back in input order while still isolating panics per task (mirroring
        // `execute_tool_calls_parallel`). Previously this used `join_all` over
        // inline futures, so a panicking handler unwound the whole call and
        // discarded every result.
        for (idx, tc) in calls.iter().enumerate() {
            let tc = tc.clone();
            let this = self.clone();
            let permits = Arc::clone(&permits);
            set.spawn(async move {
                let _permit = permits.acquire().await.ok();
                (idx, this.execute_single_tool_call(&tc).await)
            });
        }

        let mut results: Vec<Option<TextMessage>> = (0..calls.len()).map(|_| None).collect();
        while let Some(res) = set.join_next().await {
            match res {
                // Completed (or in-band-errored) call: place at its original
                // index so order is preserved.
                Ok((idx, msg)) => results[idx] = Some(msg),
                // The spawned task panicked or was cancelled before returning
                // its index, so the exact slot is unknown. Any slot still `None`
                // after the loop corresponds to such a task; fill it below with
                // the same `task_panic` message used by the parallel path.
                Err(join_err) => {
                    warn!(
                        error = %join_err,
                        "tool execution task panicked or was cancelled"
                    );
                },
            }
        }

        // Every slot whose task did not return a result gets the in-band panic
        // message, preserving the invariant that the returned length ==
        // calls.len() (matching `execute_tool_calls_parallel`).
        for slot in results.iter_mut() {
            if slot.is_none() {
                *slot = Some(TextMessage::tool(
                    serde_json::json!({
                        "error": {
                            "type": "task_panic",
                            "message": "a tool execution task panicked or was cancelled"
                        }
                    })
                    .to_string(),
                ));
            }
        }

        // All slots are populated by the loop above.
        results
            .into_iter()
            .map(|r| r.expect("every slot is filled above"))
            .collect()
    }

    /// Export a single registered tool as Tools::Function (for LLM function
    /// calling)
    pub fn export_tool_as_function(&self, name: &str) -> Option<Tools> {
        let tool = self.tools.get(name)?;
        let meta = tool.metadata();
        let schema = tool.input_schema();
        let func = Function::new(meta.name.clone(), meta.description.clone(), schema);
        Some(Tools::Function { function: func })
    }

    /// Export all registered tools as a Vec<Tools::Function>
    pub fn export_all_tools_as_functions(&self) -> Vec<Tools> {
        self.tools
            .iter()
            .map(|entry| {
                let tool = entry.value();
                let meta = tool.metadata();
                let schema = tool.input_schema();
                let func = Function::new(meta.name.clone(), meta.description.clone(), schema);
                Tools::Function { function: func }
            })
            .collect()
    }
    /// Export all registered tools with a metadata filter as Tools::Function
    pub fn export_tools_filtered<F>(&self, mut filter: F) -> Vec<Tools>
    where
        F: FnMut(&crate::toolkits::core::ToolMetadata) -> bool,
    {
        self.tools
            .iter()
            .filter(|entry| filter(entry.value().metadata()))
            .map(|entry| {
                let tool = entry.value();
                let meta = tool.metadata();
                let schema = tool.input_schema();
                let func = Function::new(meta.name.clone(), meta.description.clone(), schema);
                Tools::Function { function: func }
            })
            .collect()
    }

    #[tracing::instrument(skip(self, input))]
    async fn execute_once(
        &self,
        tool_name: &str,
        input: &serde_json::Value,
    ) -> ToolResult<serde_json::Value> {
        let tool = self
            .get_tool(tool_name)
            .ok_or_else(|| error_context().with_tool(tool_name).tool_not_found())?;

        // Honor `ToolMetadata::enabled`: a tool registered as disabled must not
        // execute. (Default `enabled = true` preserves existing behavior.)
        if !tool.metadata().enabled {
            return Err(error_context()
                .with_tool(tool_name)
                .execution_failed(format!("tool '{}' is disabled", tool_name)));
        }

        let execution_future = tool.execute_json(input.clone());

        match self.config.timeout {
            Some(timeout_duration) => match timeout(timeout_duration, execution_future).await {
                Ok(result) => result,
                Err(_) => Err(error_context()
                    .with_tool(tool_name)
                    .timeout_error(timeout_duration)),
            },
            None => execution_future.await,
        }
    }

    /// Get the executor's [`ExecutionConfig`].
    pub fn config(&self) -> &ExecutionConfig {
        &self.config
    }
}

/// Builder for creating tool executors with fluent API
pub struct ExecutorBuilder {
    config: ExecutionConfig,
    cache_config: Option<CacheConfig>,
}

#[derive(Clone)]
struct CacheConfig {
    enabled: bool,
    ttl: Duration,
    max_size: usize,
}

impl Default for ExecutorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutorBuilder {
    /// Create a new executor builder
    pub fn new() -> Self {
        Self {
            config: ExecutionConfig::default(),
            cache_config: None,
        }
    }

    /// Set timeout for tool execution
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = Some(timeout);
        self
    }

    /// Set maximum number of retries
    pub fn retries(mut self, retries: u32) -> Self {
        self.config.retry_config.max_retries = retries;
        self
    }

    /// Enable or disable logging
    pub fn logging(mut self, enabled: bool) -> Self {
        self.config.enable_logging = enabled;
        self
    }

    /// Enable tool call result caching
    pub fn enable_cache(mut self) -> Self {
        self.cache_config
            .get_or_insert(CacheConfig {
                enabled: true,
                ttl: Duration::from_secs(300),
                max_size: 1000,
            })
            .enabled = true;
        self
    }

    /// Disable tool call result caching
    pub fn disable_cache(mut self) -> Self {
        self.cache_config
            .get_or_insert(CacheConfig {
                enabled: false,
                ttl: Duration::from_secs(300),
                max_size: 1000,
            })
            .enabled = false;
        self
    }

    /// Set cache TTL
    pub fn cache_ttl(mut self, ttl: Duration) -> Self {
        let cfg = self.cache_config.get_or_insert(CacheConfig {
            enabled: true,
            ttl: Duration::from_secs(300),
            max_size: 1000,
        });
        cfg.ttl = ttl;
        self
    }

    /// Set maximum cache size
    pub fn cache_max_size(mut self, size: usize) -> Self {
        let cfg = self.cache_config.get_or_insert(CacheConfig {
            enabled: true,
            ttl: Duration::from_secs(300),
            max_size: 1000,
        });
        cfg.max_size = size;
        self
    }

    /// Build the final executor
    pub fn build(self) -> ToolExecutor {
        let cache = match self.cache_config {
            Some(cfg) => ToolCallCache::new()
                .with_enabled(cfg.enabled)
                .with_ttl(cfg.ttl)
                .with_max_size(cfg.max_size),
            None => ToolCallCache::new(),
        };

        ToolExecutor {
            tools: Arc::new(DashMap::new()),
            config: self.config,
            cache,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toolkits::core::FunctionTool;

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_delay, Duration::from_millis(100));
        assert_eq!(config.max_delay, Duration::from_secs(30));
        assert_eq!(config.backoff_multiplier, 2.0);
    }

    #[test]
    fn test_retry_config_calculate_delay() {
        let config = RetryConfig::default();

        // First attempt should have zero delay
        assert_eq!(config.calculate_delay(0), Duration::ZERO);

        // Second attempt should have initial delay
        assert_eq!(config.calculate_delay(1), Duration::from_millis(100));

        // Third attempt should double (100 * 2)
        assert_eq!(config.calculate_delay(2), Duration::from_millis(200));

        // Fourth attempt should quadruple (100 * 2^2)
        assert_eq!(config.calculate_delay(3), Duration::from_millis(400));

        // Test with exponential growth that exceeds max_delay
        let config = RetryConfig {
            max_retries: 10,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(1),
            backoff_multiplier: 3.0,
        };
        // 500ms, then 1500ms (capped at 1000ms)
        assert_eq!(config.calculate_delay(1), Duration::from_millis(500));
        assert_eq!(config.calculate_delay(2), Duration::from_secs(1));
        assert_eq!(config.calculate_delay(3), Duration::from_secs(1));
    }

    #[test]
    fn test_execution_config_default() {
        let config = ExecutionConfig::default();
        assert_eq!(config.timeout, Some(Duration::from_secs(30)));
        assert!(config.validate_parameters);
        assert!(!config.enable_logging);
        assert_eq!(config.retry_config.max_retries, 3);
    }

    #[test]
    fn test_execution_result_success() {
        let result = ExecutionResult::success(
            "test_tool".to_string(),
            serde_json::json!({"value": 42}),
            Duration::from_millis(100),
            2,
        );

        assert_eq!(result.tool_name, "test_tool");
        assert_eq!(result.result, serde_json::json!({"value": 42}));
        assert_eq!(result.duration, Duration::from_millis(100));
        assert!(result.success);
        assert!(result.error.is_none());
        assert_eq!(result.retries, 2);
        assert!(result.metadata.is_empty());
    }

    #[test]
    fn test_execution_result_failure() {
        let result = ExecutionResult::failure(
            "test_tool".to_string(),
            "Something went wrong".to_string(),
            Duration::from_millis(50),
            1,
        );

        assert_eq!(result.tool_name, "test_tool");
        assert_eq!(result.result, serde_json::Value::Null);
        assert_eq!(result.duration, Duration::from_millis(50));
        assert!(!result.success);
        assert_eq!(result.error, Some("Something went wrong".to_string()));
        assert_eq!(result.retries, 1);
        assert!(result.metadata.is_empty());
    }

    #[test]
    fn test_execution_result_with_metadata() {
        let mut result = ExecutionResult::success(
            "test_tool".to_string(),
            serde_json::json!({"value": 42}),
            Duration::from_millis(100),
            0,
        );

        result = result.with_metadata("key1", serde_json::json!("value1"));
        result = result.with_metadata("key2", serde_json::json!({"nested": true}));

        assert_eq!(result.metadata.len(), 2);
        assert_eq!(
            result.metadata.get("key1"),
            Some(&serde_json::json!("value1"))
        );
        assert_eq!(
            result.metadata.get("key2"),
            Some(&serde_json::json!({"nested": true}))
        );
    }

    #[test]
    fn test_execution_result_serialization() {
        let result = ExecutionResult::success(
            "test_tool".to_string(),
            serde_json::json!({"value": 42}),
            Duration::from_millis(100),
            0,
        );

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"tool_name\":\"test_tool\""));
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"value\":42"));
    }

    #[test]
    fn test_tool_executor_default() {
        let executor = ToolExecutor::new();
        assert_eq!(executor.tool_names().len(), 0);
        assert_eq!(executor.config.timeout, Some(Duration::from_secs(30)));
    }

    #[test]
    fn test_tool_executor_register_and_unregister() {
        let executor = ToolExecutor::new();

        // Create a simple test tool
        let tool = FunctionTool::builder("test_tool", "A test tool")
            .handler(|_args| async move { Ok(serde_json::json!({"result": "success"})) })
            .build()
            .unwrap();

        // Register the tool
        executor.add_dyn_tool(Box::new(tool)).unwrap();
        assert_eq!(executor.tool_names().len(), 1);
        assert!(executor.has_tool("test_tool"));

        // Unregister the tool
        assert!(executor.unregister("test_tool").is_ok());
        assert_eq!(executor.tool_names().len(), 0);
        assert!(!executor.has_tool("test_tool"));
    }

    #[test]
    fn test_tool_executor_duplicate_tool_returns_error() {
        let executor = ToolExecutor::new();

        let tool1 = FunctionTool::builder("duplicate_tool", "First tool")
            .handler(|_args| async move { Ok(serde_json::json!({})) })
            .build()
            .unwrap();

        let tool2 = FunctionTool::builder("duplicate_tool", "Second tool")
            .handler(|_args| async move { Ok(serde_json::json!({})) })
            .build()
            .unwrap();

        executor.add_dyn_tool(Box::new(tool1)).unwrap();

        // Adding duplicate tool should return error
        let result = executor.add_dyn_tool(Box::new(tool2));
        assert!(result.is_err());
    }

    #[test]
    fn test_tool_executor_try_add_dyn_tool() {
        let executor = ToolExecutor::new();

        let tool1 = FunctionTool::builder("test_tool", "First tool")
            .handler(|_args| async move { Ok(serde_json::json!({})) })
            .build()
            .unwrap();

        let tool2 = FunctionTool::builder("test_tool", "Second tool")
            .handler(|_args| async move { Ok(serde_json::json!({})) })
            .build()
            .unwrap();

        executor.try_add_dyn_tool(Box::new(tool1));
        executor.try_add_dyn_tool(Box::new(tool2));

        // Only one tool should be registered (second should be ignored)
        assert_eq!(executor.tool_names().len(), 1);
        assert!(executor.has_tool("test_tool"));
    }

    #[test]
    fn test_tool_executor_unregister_nonexistent_tool() {
        let executor = ToolExecutor::new();
        let result = executor.unregister("nonexistent_tool");
        assert!(result.is_err());
    }

    #[test]
    fn test_tool_executor_input_schema() {
        let executor = ToolExecutor::new();

        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        });

        let tool = FunctionTool::builder("test_tool", "A test tool")
            .schema(schema)
            .handler(|_args| async move { Ok(serde_json::json!({})) })
            .build()
            .unwrap();

        executor.add_dyn_tool(Box::new(tool)).unwrap();

        let retrieved_schema = executor.input_schema("test_tool");
        assert!(retrieved_schema.is_some());
        let retrieved = retrieved_schema.unwrap();

        // Check that schema contains expected properties
        assert_eq!(retrieved["type"], "object");
        assert_eq!(retrieved["properties"]["name"]["type"], "string");
        // additionalProperties is automatically set by FunctionToolBuilder
        assert_eq!(retrieved["additionalProperties"], false);
    }

    #[test]
    fn test_tool_executor_input_schema_nonexistent() {
        let executor = ToolExecutor::new();
        let schema = executor.input_schema("nonexistent");
        assert!(schema.is_none());
    }

    #[test]
    fn test_tool_executor_tool_names() {
        let executor = ToolExecutor::new();

        let tool1 = FunctionTool::builder("tool1", "First tool")
            .handler(|_args| async move { Ok(serde_json::json!({})) })
            .build()
            .unwrap();

        let tool2 = FunctionTool::builder("tool2", "Second tool")
            .handler(|_args| async move { Ok(serde_json::json!({})) })
            .build()
            .unwrap();

        let tool3 = FunctionTool::builder("tool3", "Third tool")
            .handler(|_args| async move { Ok(serde_json::json!({})) })
            .build()
            .unwrap();

        executor.add_dyn_tool(Box::new(tool1)).unwrap();
        executor.add_dyn_tool(Box::new(tool2)).unwrap();
        executor.add_dyn_tool(Box::new(tool3)).unwrap();

        let names = executor.tool_names();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"tool1".to_string()));
        assert!(names.contains(&"tool2".to_string()));
        assert!(names.contains(&"tool3".to_string()));
    }

    #[tokio::test]
    async fn test_tool_executor_execute_success() {
        let executor = ToolExecutor::new();

        let tool = FunctionTool::builder("add_tool", "Add two numbers")
            .property("a", serde_json::json!({"type": "number"}))
            .property("b", serde_json::json!({"type": "number"}))
            .handler(|args| async move {
                let a = args
                    .get("a")
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or(0);
                let b = args
                    .get("b")
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or(0);
                Ok(serde_json::json!({"result": a + b}))
            })
            .build()
            .unwrap();

        executor.add_dyn_tool(Box::new(tool)).unwrap();

        let input = serde_json::json!({"a": 5, "b": 3});
        let result = executor.execute("add_tool", input).await.unwrap();

        assert!(result.success);
        assert_eq!(result.tool_name, "add_tool");
        assert_eq!(result.result, serde_json::json!({"result": 8}));
        assert_eq!(result.retries, 0);
    }

    #[tokio::test]
    async fn test_tool_executor_execute_failure() {
        let executor = ToolExecutor::new();

        let tool = FunctionTool::builder("failing_tool", "Always fails")
            .handler(|_args| async move {
                Err(error_context()
                    .with_tool("failing_tool")
                    .execution_failed("Intentional failure"))
            })
            .build()
            .unwrap();

        executor.add_dyn_tool(Box::new(tool)).unwrap();

        let input = serde_json::json!({});
        let result = executor.execute("failing_tool", input).await.unwrap();

        assert!(!result.success);
        assert_eq!(result.tool_name, "failing_tool");
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_tool_executor_execute_nonexistent_tool() {
        let executor = ToolExecutor::new();
        let input = serde_json::json!({});
        let result = executor.execute("nonexistent_tool", input).await.unwrap();

        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_tool_executor_execute_simple_success() {
        let executor = ToolExecutor::new();

        let tool = FunctionTool::builder("echo_tool", "Echo input")
            .property("message", serde_json::json!({"type": "string"}))
            .handler(|args| async move { Ok(args) })
            .build()
            .unwrap();

        executor.add_dyn_tool(Box::new(tool)).unwrap();

        let input = serde_json::json!({"message": "hello"});
        let result = executor.execute_simple("echo_tool", input).await.unwrap();

        assert_eq!(result, serde_json::json!({"message": "hello"}));
    }

    #[tokio::test]
    async fn test_tool_executor_execute_simple_failure() {
        let executor = ToolExecutor::new();

        let tool = FunctionTool::builder("failing_tool", "Always fails")
            .handler(|_args| async move {
                Err(error_context()
                    .with_tool("failing_tool")
                    .execution_failed("Intentional failure"))
            })
            .build()
            .unwrap();

        executor.add_dyn_tool(Box::new(tool)).unwrap();

        let input = serde_json::json!({});
        let result = executor.execute_simple("failing_tool", input).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_tool_executor_timeout() {
        let executor = ToolExecutor::builder()
            .timeout(Duration::from_millis(100))
            .build();

        let tool = FunctionTool::builder("slow_tool", "Slow tool")
            .handler(|_args| async move {
                tokio::time::sleep(Duration::from_secs(1)).await;
                Ok(serde_json::json!({"done": true}))
            })
            .build()
            .unwrap();

        executor.add_dyn_tool(Box::new(tool)).unwrap();

        let input = serde_json::json!({});
        let result = executor.execute("slow_tool", input).await.unwrap();

        assert!(!result.success);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("Timeout"));
    }

    #[tokio::test]
    async fn test_tool_executor_retry() {
        let executor = ToolExecutor::builder()
            .retries(2)
            .timeout(Duration::from_secs(30))
            .build();

        let attempt_counter = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let counter_clone = attempt_counter.clone();

        let tool = FunctionTool::builder("flaky_tool", "Flaky tool")
            .handler(move |_args| {
                let counter = counter_clone.clone();
                async move {
                    let attempts = counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    if attempts < 2 {
                        Err(error_context()
                            .with_tool("flaky_tool")
                            .execution_failed("Temporary failure"))
                    } else {
                        Ok(serde_json::json!({"attempts": attempts + 1}))
                    }
                }
            })
            .build()
            .unwrap();

        executor.add_dyn_tool(Box::new(tool)).unwrap();

        let input = serde_json::json!({});
        let result = executor.execute("flaky_tool", input).await.unwrap();

        assert!(result.success);
        assert_eq!(result.retries, 2);
    }

    #[test]
    fn test_executor_builder_default() {
        let builder = ExecutorBuilder::new();
        assert_eq!(builder.config.timeout, Some(Duration::from_secs(30)));
        assert_eq!(builder.config.retry_config.max_retries, 3);
    }

    #[test]
    fn test_executor_builder_timeout() {
        let builder = ExecutorBuilder::new().timeout(Duration::from_secs(60));
        assert_eq!(builder.config.timeout, Some(Duration::from_secs(60)));
    }

    #[test]
    fn test_executor_builder_retries() {
        let builder = ExecutorBuilder::new().retries(5);
        assert_eq!(builder.config.retry_config.max_retries, 5);
    }

    #[test]
    fn test_executor_builder_logging() {
        let builder = ExecutorBuilder::new().logging(true);
        assert!(builder.config.enable_logging);
    }

    #[test]
    fn test_executor_builder_build() {
        let executor = ExecutorBuilder::new()
            .timeout(Duration::from_secs(60))
            .retries(5)
            .logging(true)
            .build();

        assert_eq!(executor.config.timeout, Some(Duration::from_secs(60)));
        assert_eq!(executor.config.retry_config.max_retries, 5);
        assert!(executor.config.enable_logging);
    }

    #[test]
    fn test_executor_builder_chainable() {
        let builder = ExecutorBuilder::new()
            .timeout(Duration::from_secs(45))
            .retries(3)
            .logging(false)
            .timeout(Duration::from_secs(50))
            .retries(4)
            .logging(true);

        assert_eq!(builder.config.timeout, Some(Duration::from_secs(50)));
        assert_eq!(builder.config.retry_config.max_retries, 4);
        assert!(builder.config.enable_logging);
    }

    #[test]
    fn test_export_tool_as_function() {
        let executor = ToolExecutor::new();

        let tool = FunctionTool::builder("greet_tool", "Greet someone")
            .handler(|_args| async move { Ok(serde_json::json!({"greeting": "hello"})) })
            .build()
            .unwrap();

        executor.add_dyn_tool(Box::new(tool)).unwrap();

        let exported = executor.export_tool_as_function("greet_tool");
        assert!(exported.is_some());

        if let Some(Tools::Function { function }) = exported {
            assert_eq!(function.name, "greet_tool");
            assert_eq!(function.description, "Greet someone");
            // Schema is auto-generated with default values
            assert!(function.parameters.is_some());
        } else {
            panic!("Expected Tools::Function");
        }
    }

    #[test]
    fn test_export_tool_as_function_nonexistent() {
        let executor = ToolExecutor::new();
        let exported = executor.export_tool_as_function("nonexistent");
        assert!(exported.is_none());
    }

    #[test]
    fn test_export_all_tools_as_functions() {
        let executor = ToolExecutor::new();

        let tool1 = FunctionTool::builder("tool1", "First tool")
            .handler(|_args| async move { Ok(serde_json::json!({})) })
            .build()
            .unwrap();

        let tool2 = FunctionTool::builder("tool2", "Second tool")
            .handler(|_args| async move { Ok(serde_json::json!({})) })
            .build()
            .unwrap();

        executor.add_dyn_tool(Box::new(tool1)).unwrap();
        executor.add_dyn_tool(Box::new(tool2)).unwrap();

        let exported = executor.export_all_tools_as_functions();
        assert_eq!(exported.len(), 2);

        let names: Vec<_> = exported
            .iter()
            .filter_map(|t| match t {
                Tools::Function { function } => Some(function.name.clone()),
                _ => None,
            })
            .collect();

        assert!(names.contains(&"tool1".to_string()));
        assert!(names.contains(&"tool2".to_string()));
    }

    #[test]
    fn test_export_tools_filtered() {
        let executor = ToolExecutor::new();

        let tool1 = FunctionTool::builder("math_tool", "Math operations")
            .metadata(|m| m.version("1.0.0"))
            .handler(|_args| async move { Ok(serde_json::json!({})) })
            .build()
            .unwrap();

        let tool2 = FunctionTool::builder("text_tool", "Text operations")
            .metadata(|m| m.version("2.0.0"))
            .handler(|_args| async move { Ok(serde_json::json!({})) })
            .build()
            .unwrap();

        executor.add_dyn_tool(Box::new(tool1)).unwrap();
        executor.add_dyn_tool(Box::new(tool2)).unwrap();

        let exported = executor.export_tools_filtered(|meta| meta.version == "1.0.0");
        assert_eq!(exported.len(), 1);

        if let Some(Tools::Function { function }) = exported.first() {
            assert_eq!(function.name, "math_tool");
        } else {
            panic!("Expected Tools::Function");
        }
    }

    #[test]
    fn test_execution_result_metadata_serialization() {
        let result = ExecutionResult::success(
            "test_tool".to_string(),
            serde_json::json!({"value": 42}),
            Duration::from_millis(100),
            0,
        )
        .with_metadata("key1", serde_json::json!("value1"))
        .with_metadata("key2", serde_json::json!(123));

        let json = serde_json::to_string(&result).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["metadata"]["key1"], "value1");
        assert_eq!(parsed["metadata"]["key2"], 123);
    }

    #[test]
    fn test_execution_result_timestamp() {
        let before = std::time::SystemTime::now();
        let result = ExecutionResult::success(
            "test_tool".to_string(),
            serde_json::json!({"value": 42}),
            Duration::from_millis(100),
            0,
        );
        let after = std::time::SystemTime::now();

        assert!(result.timestamp >= before && result.timestamp <= after);
    }

    #[tokio::test]
    async fn test_tool_executor_no_retry_for_non_retryable_error() {
        let executor = ToolExecutor::builder()
            .retries(3)
            .timeout(Duration::from_secs(30))
            .build();

        let attempt_counter = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let counter_clone = attempt_counter.clone();

        // ToolNotFound is not retryable, so it should fail immediately without retries
        let tool = FunctionTool::builder("not_found_tool", "Not found tool")
            .handler(move |_args| {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Err(error_context()
                        .with_tool("not_found_tool")
                        .invalid_parameters("Invalid parameters"))
                }
            })
            .build()
            .unwrap();

        executor.add_dyn_tool(Box::new(tool)).unwrap();

        let input = serde_json::json!({});
        let result = executor.execute("not_found_tool", input).await.unwrap();

        assert!(!result.success);
        assert_eq!(result.retries, 0); // Should not have retried
        // Should have been called exactly once
        assert_eq!(attempt_counter.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_execute_tool_calls_ordered_preserves_order() {
        use crate::model::chat_base_response::{ToolCallMessage, ToolFunction};

        let executor = ToolExecutor::new();

        // Register two tools that return different results
        let tool1 = FunctionTool::builder("tool_a", "First tool")
            .property("n", serde_json::json!({"type": "number"}))
            .handler(|args| async move {
                let n = args
                    .get("n")
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or(0);
                Ok(serde_json::json!({"tool": "a", "n": n}))
            })
            .build()
            .unwrap();

        let tool2 = FunctionTool::builder("tool_b", "Second tool")
            .property("n", serde_json::json!({"type": "number"}))
            .handler(|args| async move {
                let n = args
                    .get("n")
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or(0);
                Ok(serde_json::json!({"tool": "b", "n": n}))
            })
            .build()
            .unwrap();

        executor.add_dyn_tool(Box::new(tool1)).unwrap();
        executor.add_dyn_tool(Box::new(tool2)).unwrap();

        let calls = vec![
            ToolCallMessage {
                id: Some("call_1".to_string()),
                type_: Some("function".to_string()),
                function: Some(ToolFunction {
                    name: Some("tool_a".to_string()),
                    arguments: Some(r#"{"n": 1}"#.to_string()),
                }),
                mcp: None,
            },
            ToolCallMessage {
                id: Some("call_2".to_string()),
                type_: Some("function".to_string()),
                function: Some(ToolFunction {
                    name: Some("tool_b".to_string()),
                    arguments: Some(r#"{"n": 2}"#.to_string()),
                }),
                mcp: None,
            },
        ];

        let results = executor.execute_tool_calls_ordered(&calls).await;
        assert_eq!(results.len(), 2);
        // Verify ordering: first result should be from tool_a, second from tool_b
        let first = &results[0];
        let first_content = match first {
            TextMessage::Tool { content, .. } => content.clone(),
            _ => panic!("Expected Tool message"),
        };
        let parsed1: serde_json::Value = serde_json::from_str(&first_content).unwrap();
        // Check both tools since order in the content identifies which tool ran
        assert!(parsed1.get("tool").is_some());
        assert!(parsed1["n"].as_i64() == Some(1));

        let second = &results[1];
        let second_content = match second {
            TextMessage::Tool { content, .. } => content.clone(),
            _ => panic!("Expected Tool message"),
        };
        let parsed2: serde_json::Value = serde_json::from_str(&second_content).unwrap();
        assert!(parsed2.get("tool").is_some());
        assert!(parsed2["n"].as_i64() == Some(2));
    }

    #[tokio::test]
    async fn test_execute_tool_calls_parallel_returns_all() {
        use crate::model::chat_base_response::{ToolCallMessage, ToolFunction};

        let executor = ToolExecutor::new();

        let tool1 = FunctionTool::builder("parallel_a", "First parallel tool")
            .property("n", serde_json::json!({"type": "number"}))
            .handler(|args| async move {
                let n = args
                    .get("n")
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or(0);
                Ok(serde_json::json!({"tool": "a", "n": n}))
            })
            .build()
            .unwrap();

        let tool2 = FunctionTool::builder("parallel_b", "Second parallel tool")
            .property("n", serde_json::json!({"type": "number"}))
            .handler(|args| async move {
                let n = args
                    .get("n")
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or(0);
                Ok(serde_json::json!({"tool": "b", "n": n}))
            })
            .build()
            .unwrap();

        executor.add_dyn_tool(Box::new(tool1)).unwrap();
        executor.add_dyn_tool(Box::new(tool2)).unwrap();

        let calls = vec![
            ToolCallMessage {
                id: Some("call_1".to_string()),
                type_: Some("function".to_string()),
                function: Some(ToolFunction {
                    name: Some("parallel_a".to_string()),
                    arguments: Some(r#"{"n": 1}"#.to_string()),
                }),
                mcp: None,
            },
            ToolCallMessage {
                id: Some("call_2".to_string()),
                type_: Some("function".to_string()),
                function: Some(ToolFunction {
                    name: Some("parallel_b".to_string()),
                    arguments: Some(r#"{"n": 2}"#.to_string()),
                }),
                mcp: None,
            },
        ];

        let results = executor.execute_tool_calls_parallel(&calls).await;
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_execute_works_with_cache_disabled() {
        // Regression guard for the lazy cache-key path: with caching disabled,
        // execute() must not build a key / touch the cache, yet still succeed.
        let executor = ToolExecutor::builder().disable_cache().build();

        let tool = FunctionTool::builder("echo", "echo input")
            .property("x", serde_json::json!({"type": "number"}))
            .handler(|args| async move { Ok(args) })
            .build()
            .unwrap();
        executor.add_dyn_tool(Box::new(tool)).unwrap();

        let result = executor
            .execute("echo", serde_json::json!({"x": 1}))
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(result.result, serde_json::json!({"x": 1}));
        // Cache stays empty (disabled).
        assert_eq!(executor.cache_stats().total_entries, 0);
    }
}
