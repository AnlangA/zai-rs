//! Tool registry and executor with caching, retries, and bounded concurrency.

use std::{
    collections::HashMap,
    panic::AssertUnwindSafe,
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    time::Instant,
};

use dashmap::{DashMap, mapref::entry::Entry};
use futures_util::{FutureExt, StreamExt};
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};
use tokio::time::timeout;
use tracing::warn;

use super::{
    cache::{CacheKey, ToolCallCache},
    core::{FunctionTool, ToolHandler, validate_tool_name},
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

mod types;

pub use super::core::ToolRegistration;
pub use types::{ExecutionConfig, ExecutionResult, ExecutorBuilder, RetryConfig};

/// Cap on how many tool calls run concurrently in
/// [`execute_tool_calls_parallel`](ToolExecutor::execute_tool_calls_parallel)
/// / [`execute_tool_calls_ordered`](ToolExecutor::execute_tool_calls_ordered).
///
/// Prevents a model that emits many tool calls in one turn from fanning them
/// all out at once and overwhelming downstream services.
const MAX_CONCURRENT_TOOL_CALLS: usize = 8;

fn tool_message(payload: serde_json::Value, id: Option<&str>) -> TextMessage {
    let content = payload.to_string();
    match id {
        Some(id) => TextMessage::tool_with_id(content, id),
        None => TextMessage::tool(content),
    }
}

fn tool_error_message(error_type: &str, message: &str, id: Option<&str>) -> TextMessage {
    tool_message(
        serde_json::json!({"error": {"type": error_type, "message": message}}),
        id,
    )
}

fn task_panic_tool_message(id: Option<&str>) -> TextMessage {
    tool_error_message("task_panic", "a tool execution future panicked", id)
}

fn sort_exported_tools(tools: &mut [Tools]) {
    fn function_name(tool: &Tools) -> Option<&str> {
        match tool {
            Tools::Function { function } => Some(&function.name),
            _ => None,
        }
    }
    tools.sort_unstable_by(|left, right| function_name(left).cmp(&function_name(right)));
}

fn export_enabled_tool(tool: &dyn DynTool) -> Option<Tools> {
    let metadata = tool.metadata();
    metadata.is_enabled().then(|| Tools::Function {
        function: Function::new(metadata.name(), metadata.description(), tool.input_schema()),
    })
}

/// Tool registry and executor with optional result caching.
#[derive(Clone)]
pub struct ToolExecutor {
    tools: Arc<DashMap<String, RegisteredTool>>,
    next_generation: Arc<AtomicU64>,
    registry_mutation_lock: Arc<Mutex<()>>,
    config: ExecutionConfig,
    cache: ToolCallCache,
    cache_flights: Arc<DashMap<CacheKey, Arc<FlightGate>>>,
    cache_global_epoch: Arc<AtomicU64>,
    cache_epoch_fence: Arc<RwLock<()>>,
}

#[derive(Clone, Copy)]
struct CacheEpoch {
    global: u64,
    tool: u64,
}

struct RegisteredTool {
    tool: Arc<dyn DynTool>,
    generation: u64,
    cache_epoch: Arc<AtomicU64>,
}

struct FlightGate {
    lock: Arc<AsyncMutex<()>>,
    users: AtomicUsize,
}

/// Owns one registered user of a keyed cache-miss admission slot.
///
/// Registration happens before waiting for the mutex, so cancellation at any
/// await point decrements the user count and removes an idle slot.
struct CacheFlight {
    flights: Arc<DashMap<CacheKey, Arc<FlightGate>>>,
    key: CacheKey,
    gate: Arc<FlightGate>,
    guard: Option<OwnedMutexGuard<()>>,
}

impl CacheFlight {
    async fn enter(flights: Arc<DashMap<CacheKey, Arc<FlightGate>>>, key: CacheKey) -> Self {
        let entry = flights.entry(key.clone()).or_insert_with(|| {
            Arc::new(FlightGate {
                lock: Arc::new(AsyncMutex::new(())),
                users: AtomicUsize::new(0),
            })
        });
        // Increment while the DashMap entry guard is alive. A concurrent
        // last-user cleanup therefore cannot remove this gate between lookup
        // and registration.
        entry.users.fetch_add(1, Ordering::AcqRel);
        let gate = Arc::clone(entry.value());
        drop(entry);
        let mut flight = Self {
            flights,
            key,
            gate: Arc::clone(&gate),
            guard: None,
        };
        flight.guard = Some(gate.lock.clone().lock_owned().await);
        flight
    }
}

impl Drop for CacheFlight {
    fn drop(&mut self) {
        // Unlock first so a registered waiter can advance immediately.
        self.guard.take();
        let previous = self.gate.users.fetch_sub(1, Ordering::AcqRel);
        debug_assert!(previous > 0, "cache-flight user count underflow");
        if previous == 1 {
            self.flights.remove_if(&self.key, |_key, current| {
                Arc::ptr_eq(current, &self.gate) && current.users.load(Ordering::Acquire) == 0
            });
        }
    }
}

impl std::fmt::Debug for ToolExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tool_count = self.tools.len();
        let cache_enabled = self.cache.enabled();
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
            next_generation: Arc::new(AtomicU64::new(0)),
            registry_mutation_lock: Arc::new(Mutex::new(())),
            config: ExecutionConfig::default(),
            // Tool purity is unknown at registration time, so caching is an
            // explicit opt-in.
            cache: ToolCallCache::new().with_enabled(false),
            cache_flights: Arc::new(DashMap::new()),
            cache_global_epoch: Arc::new(AtomicU64::new(0)),
            cache_epoch_fence: Arc::new(RwLock::new(())),
        }
    }

    /// Create an executor builder for fluent API
    pub fn builder() -> ExecutorBuilder {
        ExecutorBuilder::new()
    }

    /// Clear the cache
    pub fn clear_cache(&self) {
        let _fence = self
            .cache_epoch_fence
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        saturating_increment_epoch(&self.cache_global_epoch);
        self.cache.clear();
    }

    /// Invalidate cache for a specific tool
    pub fn invalidate_cache_for_tool(&self, tool_name: &str) {
        let _fence = self
            .cache_epoch_fence
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(registered) = self.tools.get(tool_name) {
            saturating_increment_epoch(&registered.cache_epoch);
        }
        self.cache.invalidate_tool(tool_name);
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> super::CacheStats {
        self.cache.stats()
    }

    /// Chain-friendly: add a dynamic tool, returns error if already registered
    pub fn add_dyn_tool(&self, tool: Box<dyn DynTool>) -> ToolResult<&Self> {
        let name = tool.name().to_string();
        validate_tool_name(&name)?;
        let mutation = self
            .registry_mutation_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match self.tools.entry(name.clone()) {
            Entry::Occupied(_) => {
                return Err(ToolError::RegistrationError {
                    message: format!("Tool '{name}' is already registered").into(),
                });
            },
            Entry::Vacant(entry) => {
                let generation = self.next_generation.fetch_add(1, Ordering::Relaxed);
                entry.insert(RegisteredTool {
                    tool: Arc::from(tool),
                    generation,
                    cache_epoch: Arc::new(AtomicU64::new(0)),
                });
            },
        }
        drop(mutation);
        // Remove entries from a prior registration with this name after the
        // registry shard guard has been released.
        self.invalidate_cache_for_tool(&name);
        Ok(self)
    }

    /// Unregister a tool
    pub fn unregister(&self, name: &str) -> ToolResult<()> {
        let mutation = self
            .registry_mutation_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if self.tools.remove(name).is_none() {
            return Err(error_context().with_tool(name).tool_not_found());
        }
        drop(mutation);
        self.invalidate_cache_for_tool(name);
        Ok(())
    }

    /// Get input schema for a tool
    pub fn input_schema(&self, name: &str) -> Option<serde_json::Value> {
        self.tools.get(name).map(|entry| entry.tool.input_schema())
    }

    /// Check if tool exists
    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// List tool names in deterministic lexicographic order.
    pub fn tool_names(&self) -> Vec<String> {
        let mut names: Vec<_> = self.tools.iter().map(|entry| entry.key().clone()).collect();
        names.sort_unstable();
        names
    }

    fn get_tool(&self, name: &str) -> Option<(Arc<dyn DynTool>, u64, Arc<AtomicU64>)> {
        self.tools.get(name).map(|entry| {
            (
                Arc::clone(&entry.tool),
                entry.generation,
                Arc::clone(&entry.cache_epoch),
            )
        })
    }

    fn cache_if_still_registered(
        &self,
        tool_name: &str,
        generation: u64,
        tool_cache_epoch: &AtomicU64,
        cache_epoch: CacheEpoch,
        key: CacheKey,
        result: serde_json::Value,
    ) {
        // This read fence makes clear/invalidate linearizable with insertion:
        // either the value lands before the mutation and is removed by it, or
        // the epoch has changed and the stale execution cannot write back.
        let _fence = self
            .cache_epoch_fence
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if self.cache_global_epoch.load(Ordering::Acquire) != cache_epoch.global
            || tool_cache_epoch.load(Ordering::Acquire) != cache_epoch.tool
        {
            return;
        }
        // Keep the map guard until after insertion. An unregister operation
        // either happens first (and this write is skipped) or happens after
        // the write and invalidates it, so an old in-flight call cannot leave
        // an unreachable stale entry behind.
        if let Some(registered) = self.tools.get(tool_name)
            && registered.generation == generation
        {
            self.cache.insert(key, result, None);
        }
    }

    fn current_cache_epoch(&self, tool_cache_epoch: &AtomicU64) -> CacheEpoch {
        CacheEpoch {
            global: self.cache_global_epoch.load(Ordering::Acquire),
            tool: tool_cache_epoch.load(Ordering::Acquire),
        }
    }

    /// Execute a tool with caching, timeout, and exponential backoff.
    ///
    /// Tool-level failures are returned as `Ok(ExecutionResult)` with its
    /// `success` field set to `false`. Use [`ToolExecutor::execute_simple`]
    /// when failures should be returned as [`ToolError`] values instead.
    #[tracing::instrument(skip(self, input))]
    pub async fn execute(
        &self,
        tool_name: &str,
        input: serde_json::Value,
    ) -> ToolResult<ExecutionResult> {
        let start_time = Instant::now();
        let mut retries = 0;
        let retry_config = &self.config.retry_config;

        let Some((tool, generation, tool_cache_epoch)) = self.get_tool(tool_name) else {
            let error = error_context().with_tool(tool_name).tool_not_found();
            return Ok(ExecutionResult::failure(
                tool_name.to_string(),
                error.to_string(),
                start_time.elapsed(),
                0,
            ));
        };
        if !tool.metadata().is_enabled() {
            let error = error_context()
                .with_tool(tool_name)
                .execution_failed(format!("tool '{tool_name}' is disabled"));
            return Ok(ExecutionResult::failure(
                tool_name.to_string(),
                error.to_string(),
                start_time.elapsed(),
                0,
            ));
        }
        let execution_policy = tool.metadata().execution_policy();

        // Registration generation prevents an in-flight call from repopulating
        // a cache entry that a later tool with the same name could consume.
        let cache_key = if self.cache.can_store() && execution_policy.allows_cache() {
            Some(CacheKey::for_generation(
                tool_name.to_string(),
                &input,
                generation,
            ))
        } else {
            None
        };
        if let Some(ref key) = cache_key
            && let Some(cached_result) = self.cache.peek(key)
        {
            self.cache.record_hit();
            let duration = start_time.elapsed();
            return Ok(ExecutionResult::success(
                tool_name.to_string(),
                cached_result,
                duration,
                retries,
            )
            .with_cache_hit());
        }
        // Collapse concurrent misses for the same tool generation and
        // canonical input. A second lookup after admission observes the
        // leader's successful result without serializing hot-cache hits.
        let _cache_flight = if let Some(ref key) = cache_key {
            let flight = CacheFlight::enter(Arc::clone(&self.cache_flights), key.clone()).await;
            if let Some(cached_result) = self.cache.peek(key) {
                self.cache.record_hit();
                let duration = start_time.elapsed();
                return Ok(ExecutionResult::success(
                    tool_name.to_string(),
                    cached_result,
                    duration,
                    retries,
                )
                .with_cache_hit());
            }
            self.cache.record_miss();
            Some(flight)
        } else {
            None
        };
        // Capture the invalidation generation only after admission and the
        // second miss. A waiter queued before clear/invalidate therefore
        // executes in the new generation and can repopulate the cache.
        let cache_epoch = cache_key
            .as_ref()
            .map(|_| self.current_cache_epoch(&tool_cache_epoch));

        loop {
            match self.execute_once(&tool, tool_name, &input).await {
                Ok(result) => {
                    let duration = start_time.elapsed();
                    if let Some(key) = cache_key {
                        self.cache_if_still_registered(
                            tool_name,
                            generation,
                            &tool_cache_epoch,
                            cache_epoch.expect("cache key always has an epoch"),
                            key,
                            result.clone(),
                        );
                    }

                    return Ok(ExecutionResult::success(
                        tool_name.to_string(),
                        result,
                        duration,
                        retries,
                    ));
                },
                Err(error) => {
                    // Both gates are required: the error must be transient and
                    // the tool author must have declared the complete call
                    // idempotent.
                    if !execution_policy.allows_retry() || !error.is_retryable() {
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

                    warn!(attempt = retries, "Tool execution failed, retrying");

                    // Use exponential backoff
                    let delay = retry_config.calculate_delay(retries);
                    tokio::time::sleep(delay).await;
                },
            }
        }
    }

    /// Execute a tool and return its JSON value, converting an unsuccessful
    /// execution into a [`ToolError`].
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

    /// Register function specifications loaded from `.json` files in `dir`.
    ///
    /// Each file may contain a direct function object or an OpenAI-style
    /// `{ "type": "function", "function": ... }` wrapper. `handlers` maps
    /// function names to implementations. When `strict` is `false`, files
    /// without a matching handler are skipped; otherwise they cause an error.
    ///
    /// This compatibility API always attaches the safe
    /// [`ToolExecutionPolicy::default`](crate::toolkits::core::ToolExecutionPolicy)
    /// to every handler. Use
    /// [`Self::add_functions_from_dir_with_registrations`] for explicit
    /// cache/retry eligibility.
    ///
    /// All JSON files and selected schemas are validated before registry
    /// mutation. Duplicate names in the directory or conflicts with existing
    /// tools reject the whole batch; no selected tool is registered.
    pub fn add_functions_from_dir_with_registry(
        &self,
        dir: impl AsRef<std::path::Path>,
        handlers: &HashMap<String, ToolHandler>,
        strict: bool,
    ) -> ToolResult<Vec<String>> {
        let registrations = handlers
            .iter()
            .map(|(name, handler)| (name.clone(), ToolRegistration::new(Arc::clone(handler))))
            .collect();
        self.add_functions_from_dir_with_registrations(dir, &registrations, strict)
    }

    /// Register directory-loaded functions with trusted local effect policies.
    ///
    /// Each `.json` file may contain either a direct function object or an
    /// OpenAI-style `{ "type": "function", "function": ... }` wrapper.
    /// Model-facing JSON controls only the function name, description, and
    /// parameters. Fields that resemble an execution policy are ignored:
    /// cache/retry eligibility comes exclusively from [`ToolRegistration`].
    ///
    /// When `strict` is `true`, every valid JSON specification must have a
    /// matching registration. When it is `false`, missing registrations are
    /// skipped; registrations without a matching file are always ignored.
    /// Duplicate function names in the directory and names already present in
    /// this executor are errors. Files are parsed, schemas are compiled, and
    /// conflicts are checked before the batch is committed, so an error does
    /// not leave a partially registered batch.
    ///
    /// Returns registered names in deterministic file-path order.
    pub fn add_functions_from_dir_with_registrations(
        &self,
        dir: impl AsRef<std::path::Path>,
        registrations: &HashMap<String, ToolRegistration>,
        strict: bool,
    ) -> ToolResult<Vec<String>> {
        let staged = Self::stage_directory_tools(dir.as_ref(), registrations, strict)?;
        self.commit_directory_tools(staged)
    }

    fn stage_directory_tools(
        dir: &std::path::Path,
        registrations: &HashMap<String, ToolRegistration>,
        strict: bool,
    ) -> ToolResult<Vec<(String, FunctionTool)>> {
        use std::fs;

        use serde_json::Value;
        let read_dir = fs::read_dir(dir).map_err(|e| {
            error_context().invalid_parameters(format!(
                "Failed to read dir {}: {}",
                dir.display(),
                e
            ))
        })?;
        let mut paths = read_dir
            .map(|entry| {
                entry.map(|entry| entry.path()).map_err(|error| {
                    error_context().invalid_parameters(format!("Dir entry error: {error}"))
                })
            })
            .collect::<ToolResult<Vec<_>>>()?;
        paths.retain(|path| {
            path.is_file()
                && path.extension().and_then(|extension| extension.to_str()) == Some("json")
        });
        paths.sort_unstable();

        let mut first_path_by_name = HashMap::new();
        let mut staged = Vec::new();
        for path in paths {
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

            let (name, description, parameters) =
                crate::toolkits::core::parse_function_spec_details(&spec).map_err(|e| {
                    error_context().invalid_parameters(format!(
                        "Failed to parse spec {}: {}",
                        path.display(),
                        e
                    ))
                })?;

            if let Some(first_path) = first_path_by_name.insert(name.clone(), path.clone()) {
                return Err(ToolError::RegistrationError {
                    message: format!(
                        "Duplicate function name '{name}' in {} and {}; directory batch was not changed",
                        first_path.display(),
                        path.display()
                    )
                    .into(),
                });
            }

            let registration = match registrations.get(&name) {
                Some(registration) => registration,
                None => {
                    if strict {
                        return Err(error_context().invalid_parameters(format!(
                            "No handler registered for function '{}' (file {})",
                            name,
                            path.display()
                        )));
                    }
                    continue;
                },
            };

            let mut builder = FunctionTool::builder(name.clone(), description)
                .execution_policy(registration.execution_policy());
            if let Some(p) = parameters {
                builder = builder.schema(p);
            }
            let handler = registration.handler();
            let tool = builder
                .handler(move |args| {
                    let handler = Arc::clone(&handler);
                    handler(args)
                })
                .build()?;

            staged.push((name, tool));
        }
        Ok(staged)
    }

    fn commit_directory_tools(
        &self,
        staged: Vec<(String, FunctionTool)>,
    ) -> ToolResult<Vec<String>> {
        let mutation = self
            .registry_mutation_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some((name, _)) = staged
            .iter()
            .find(|(name, _)| self.tools.contains_key(name))
        {
            return Err(ToolError::RegistrationError {
                message: format!(
                    "Tool '{name}' is already registered; directory batch was not changed"
                )
                .into(),
            });
        }

        let mut added = Vec::with_capacity(staged.len());
        for (name, tool) in staged {
            let generation = self.next_generation.fetch_add(1, Ordering::Relaxed);
            let tool: Arc<dyn DynTool> = Arc::new(tool);
            let previous = self.tools.insert(
                name.clone(),
                RegisteredTool {
                    tool,
                    generation,
                    cache_epoch: Arc::new(AtomicU64::new(0)),
                },
            );
            debug_assert!(previous.is_none(), "directory conflict preflight drifted");
            added.push(name);
        }
        drop(mutation);

        for name in &added {
            self.invalidate_cache_for_tool(name);
        }
        Ok(added)
    }

    async fn execute_single_tool_call(&self, tc: &ToolCallMessage) -> TextMessage {
        let id = tc.id();
        let Some(function) = tc.function() else {
            return tool_error_message("missing_function", "tool_call.function is missing", id);
        };
        let name = function.name();
        if name.trim().is_empty() {
            return tool_error_message(
                "missing_function_name",
                "tool_call.function.name is blank",
                id,
            );
        }
        let arguments = match serde_json::from_str(function.arguments()) {
            Ok(serde_json::Value::Object(arguments)) => serde_json::Value::Object(arguments),
            Ok(_) => {
                return tool_error_message(
                    "invalid_arguments",
                    "tool arguments must decode to a JSON object",
                    id,
                );
            },
            Err(error) => {
                return tool_error_message(
                    "invalid_arguments",
                    &format!("tool arguments are not valid JSON: {error}"),
                    id,
                );
            },
        };

        match self.execute_simple(name, arguments).await {
            Ok(result) => tool_message(result, id),
            Err(error) => tool_error_message("execution_failed", &error.to_string(), id),
        }
    }

    async fn execute_tool_calls_bounded(
        &self,
        calls: &[ToolCallMessage],
    ) -> Vec<(usize, TextMessage)> {
        futures_util::stream::iter(calls.iter().enumerate())
            .map(|(index, call)| async move {
                let message = match AssertUnwindSafe(self.execute_single_tool_call(call))
                    .catch_unwind()
                    .await
                {
                    Ok(message) => message,
                    Err(_) => {
                        warn!(index, "tool execution future panicked");
                        task_panic_tool_message(call.id())
                    },
                };
                (index, message)
            })
            .buffer_unordered(MAX_CONCURRENT_TOOL_CALLS)
            .collect()
            .await
    }

    /// Execute tool calls concurrently, returning one tool message per call.
    ///
    /// Results are returned in completion order; use
    /// [`Self::execute_tool_calls_ordered`] to restore input order.
    pub async fn execute_tool_calls_parallel(&self, calls: &[ToolCallMessage]) -> Vec<TextMessage> {
        self.execute_tool_calls_bounded(calls)
            .await
            .into_iter()
            .map(|(_, message)| message)
            .collect()
    }

    /// Execute tool calls concurrently and restore input order in the result.
    ///
    /// At most eight handlers run concurrently. Invalid arguments, execution
    /// errors, and handler panics are isolated to their call and encoded as a
    /// tool-error message, so one failure does not discard other results.
    pub async fn execute_tool_calls_ordered(&self, calls: &[ToolCallMessage]) -> Vec<TextMessage> {
        let mut results = self.execute_tool_calls_bounded(calls).await;
        results.sort_unstable_by_key(|(index, _)| *index);
        results.into_iter().map(|(_, message)| message).collect()
    }

    /// Export one registered tool as an LLM function definition.
    pub fn export_tool_as_function(&self, name: &str) -> Option<Tools> {
        let registered = self.tools.get(name)?;
        export_enabled_tool(registered.tool.as_ref())
    }

    /// Export all registered tools as function definitions sorted by name.
    pub fn export_all_tools_as_functions(&self) -> Vec<Tools> {
        let mut tools: Vec<_> = self
            .tools
            .iter()
            .filter_map(|entry| export_enabled_tool(entry.value().tool.as_ref()))
            .collect();
        sort_exported_tools(&mut tools);
        tools
    }
    /// Export enabled tools selected by a metadata predicate.
    pub fn export_tools_filtered<F>(&self, mut filter: F) -> Vec<Tools>
    where
        F: FnMut(&crate::toolkits::core::ToolMetadata) -> bool,
    {
        let mut tools: Vec<_> = self
            .tools
            .iter()
            .filter(|entry| filter(entry.value().tool.metadata()))
            .filter_map(|entry| export_enabled_tool(entry.value().tool.as_ref()))
            .collect();
        sort_exported_tools(&mut tools);
        tools
    }

    #[tracing::instrument(skip(self, tool, input))]
    async fn execute_once(
        &self,
        tool: &Arc<dyn DynTool>,
        tool_name: &str,
        input: &serde_json::Value,
    ) -> ToolResult<serde_json::Value> {
        let execution_future = AssertUnwindSafe(tool.execute_json(input.clone())).catch_unwind();

        let outcome = match self.config.timeout {
            Some(timeout_duration) => match timeout(timeout_duration, execution_future).await {
                Ok(result) => result,
                Err(_) => {
                    return Err(error_context()
                        .with_tool(tool_name)
                        .timeout_error(timeout_duration));
                },
            },
            None => execution_future.await,
        };

        match outcome {
            Ok(result) => result,
            Err(_) => {
                warn!("tool execution handler panicked");
                Err(ToolError::ExecutionPanicked {
                    tool: tool_name.to_string().into(),
                })
            },
        }
    }

    /// Get the executor's [`ExecutionConfig`].
    pub fn config(&self) -> &ExecutionConfig {
        &self.config
    }
}

fn saturating_increment_epoch(epoch: &AtomicU64) {
    let mut current = epoch.load(Ordering::Acquire);
    while current != u64::MAX {
        match epoch.compare_exchange_weak(current, current + 1, Ordering::AcqRel, Ordering::Acquire)
        {
            Ok(_) => break,
            Err(observed) => current = observed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    use crate::toolkits::core::{CachePolicy, FunctionTool, RetryPolicy, ToolExecutionPolicy};

    #[test]
    fn saturating_epoch_increment_stops_at_max() {
        let epoch = AtomicU64::new(0);
        saturating_increment_epoch(&epoch);
        assert_eq!(epoch.load(Ordering::Acquire), 1);

        epoch.store(u64::MAX, Ordering::Release);
        saturating_increment_epoch(&epoch);
        assert_eq!(epoch.load(Ordering::Acquire), u64::MAX);
    }

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 0);
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

        // Exponential growth remains bounded by the configured maximum.
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
        assert_eq!(config.retry_config.max_retries, 0);
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
        assert!(!result.cache_hit);
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
        assert!(!result.cache_hit);
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
            .execution_policy(ToolExecutionPolicy::new(
                CachePolicy::Never,
                RetryPolicy::Idempotent,
            ))
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
        assert_eq!(builder.config.retry_config.max_retries, 0);
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
    fn test_executor_builder_build() {
        let executor = ExecutorBuilder::new()
            .timeout(Duration::from_secs(60))
            .retries(5)
            .build();

        assert_eq!(executor.config.timeout, Some(Duration::from_secs(60)));
        assert_eq!(executor.config.retry_config.max_retries, 5);
    }

    #[test]
    fn test_executor_builder_chainable() {
        let builder = ExecutorBuilder::new()
            .timeout(Duration::from_secs(45))
            .retries(3)
            .timeout(Duration::from_secs(50))
            .retries(4);

        assert_eq!(builder.config.timeout, Some(Duration::from_secs(50)));
        assert_eq!(builder.config.retry_config.max_retries, 4);
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
            .metadata(|m| m.with_version("1.0.0"))
            .handler(|_args| async move { Ok(serde_json::json!({})) })
            .build()
            .unwrap();

        let tool2 = FunctionTool::builder("text_tool", "Text operations")
            .metadata(|m| m.with_version("2.0.0"))
            .handler(|_args| async move { Ok(serde_json::json!({})) })
            .build()
            .unwrap();

        executor.add_dyn_tool(Box::new(tool1)).unwrap();
        executor.add_dyn_tool(Box::new(tool2)).unwrap();

        let exported = executor.export_tools_filtered(|meta| meta.version() == "1.0.0");
        assert_eq!(exported.len(), 1);

        if let Some(Tools::Function { function }) = exported.first() {
            assert_eq!(function.name, "math_tool");
        } else {
            panic!("Expected Tools::Function");
        }
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
            .execution_policy(ToolExecutionPolicy::new(
                CachePolicy::Never,
                RetryPolicy::Idempotent,
            ))
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
                    name: "tool_a".to_string(),
                    arguments: r#"{"n": 1}"#.to_string(),
                }),
                mcp: None,
            },
            ToolCallMessage {
                id: Some("call_2".to_string()),
                type_: Some("function".to_string()),
                function: Some(ToolFunction {
                    name: "tool_b".to_string(),
                    arguments: r#"{"n": 2}"#.to_string(),
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
                    name: "parallel_a".to_string(),
                    arguments: r#"{"n": 1}"#.to_string(),
                }),
                mcp: None,
            },
            ToolCallMessage {
                id: Some("call_2".to_string()),
                type_: Some("function".to_string()),
                function: Some(ToolFunction {
                    name: "parallel_b".to_string(),
                    arguments: r#"{"n": 2}"#.to_string(),
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
        let executor = ToolExecutor::new();

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

    #[tokio::test]
    async fn cancelled_waiters_release_their_flight_registration() {
        let started = Arc::new(tokio::sync::Notify::new());
        let tool = FunctionTool::builder("flight_cleanup", "flight cleanup fixture")
            .execution_policy(ToolExecutionPolicy::new(
                CachePolicy::Pure,
                RetryPolicy::Never,
            ))
            .handler({
                let started = Arc::clone(&started);
                move |_| {
                    let started = Arc::clone(&started);
                    async move {
                        started.notify_one();
                        std::future::pending::<()>().await;
                        #[allow(unreachable_code)]
                        Ok(serde_json::json!({}))
                    }
                }
            })
            .build()
            .unwrap();
        let executor = ToolExecutor::builder().enable_cache().build();
        executor.add_dyn_tool(Box::new(tool)).unwrap();

        let leader = {
            let executor = executor.clone();
            tokio::spawn(async move {
                executor
                    .execute("flight_cleanup", serde_json::json!({}))
                    .await
            })
        };
        started.notified().await;
        let waiter_one = {
            let executor = executor.clone();
            tokio::spawn(async move {
                executor
                    .execute("flight_cleanup", serde_json::json!({}))
                    .await
            })
        };
        let waiter_two = {
            let executor = executor.clone();
            tokio::spawn(async move {
                executor
                    .execute("flight_cleanup", serde_json::json!({}))
                    .await
            })
        };

        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                let users = executor
                    .cache_flights
                    .iter()
                    .next()
                    .map(|entry| entry.users.load(Ordering::Acquire))
                    .unwrap_or(0);
                if users == 3 {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("waiters were not registered");

        waiter_one.abort();
        waiter_two.abort();
        assert!(waiter_one.await.unwrap_err().is_cancelled());
        assert!(waiter_two.await.unwrap_err().is_cancelled());
        assert_eq!(executor.cache_flights.len(), 1);

        leader.abort();
        assert!(leader.await.unwrap_err().is_cancelled());
        assert!(executor.cache_flights.is_empty());
    }

    #[tokio::test]
    async fn cache_clear_fences_an_in_flight_write_back() {
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let started = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let tool = FunctionTool::builder("clear_fence", "cache clear fixture")
            .execution_policy(ToolExecutionPolicy::new(
                CachePolicy::Pure,
                RetryPolicy::Never,
            ))
            .handler({
                let calls = Arc::clone(&calls);
                let started = Arc::clone(&started);
                let release = Arc::clone(&release);
                move |_| {
                    let calls = Arc::clone(&calls);
                    let started = Arc::clone(&started);
                    let release = Arc::clone(&release);
                    async move {
                        let invocation = calls.fetch_add(1, Ordering::SeqCst);
                        if invocation == 0 {
                            started.notify_one();
                            release.notified().await;
                        }
                        Ok(serde_json::json!({"invocation": invocation + 1}))
                    }
                }
            })
            .build()
            .unwrap();
        let executor = ToolExecutor::builder().enable_cache().build();
        executor.add_dyn_tool(Box::new(tool)).unwrap();

        let execution = {
            let executor = executor.clone();
            tokio::spawn(
                async move { executor.execute("clear_fence", serde_json::json!({})).await },
            )
        };
        started.notified().await;
        executor.clear_cache();
        release.notify_one();
        assert!(execution.await.unwrap().unwrap().success);
        assert_eq!(executor.cache_stats().total_entries, 0);

        let next = executor
            .execute("clear_fence", serde_json::json!({}))
            .await
            .unwrap();
        assert_eq!(next.result, serde_json::json!({"invocation": 2}));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(executor.cache_stats().total_entries, 1);
    }

    #[tokio::test]
    async fn waiter_queued_before_clear_repopulates_the_new_cache_generation() {
        let calls = Arc::new(AtomicUsize::new(0));
        let started = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let tool = FunctionTool::builder("clear_waiter", "cache clear waiter fixture")
            .execution_policy(ToolExecutionPolicy::new(
                CachePolicy::Pure,
                RetryPolicy::Never,
            ))
            .handler({
                let calls = Arc::clone(&calls);
                let started = Arc::clone(&started);
                let release = Arc::clone(&release);
                move |_| {
                    let calls = Arc::clone(&calls);
                    let started = Arc::clone(&started);
                    let release = Arc::clone(&release);
                    async move {
                        let invocation = calls.fetch_add(1, Ordering::SeqCst) + 1;
                        if invocation == 1 {
                            started.notify_one();
                            release.notified().await;
                        }
                        Ok(serde_json::json!({"invocation": invocation}))
                    }
                }
            })
            .build()
            .unwrap();
        let executor = ToolExecutor::builder().enable_cache().build();
        executor.add_dyn_tool(Box::new(tool)).unwrap();

        let leader = {
            let executor = executor.clone();
            tokio::spawn(async move {
                executor
                    .execute("clear_waiter", serde_json::json!({}))
                    .await
            })
        };
        started.notified().await;
        let waiter = {
            let executor = executor.clone();
            tokio::spawn(async move {
                executor
                    .execute("clear_waiter", serde_json::json!({}))
                    .await
            })
        };
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                let users = executor
                    .cache_flights
                    .iter()
                    .next()
                    .map(|entry| entry.users.load(Ordering::Acquire))
                    .unwrap_or(0);
                if users == 2 {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("waiter was not admitted before clear");

        executor.clear_cache();
        release.notify_one();
        assert!(leader.await.unwrap().unwrap().success);
        let waiter_result = waiter.await.unwrap().unwrap();
        assert_eq!(waiter_result.result, serde_json::json!({"invocation": 2}));

        let cached = executor
            .execute("clear_waiter", serde_json::json!({}))
            .await
            .unwrap();
        assert!(cached.cache_hit);
        assert_eq!(cached.result, serde_json::json!({"invocation": 2}));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn per_tool_invalidation_fences_an_in_flight_write_back() {
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let started = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let tool = FunctionTool::builder("invalidate_fence", "cache invalidation fixture")
            .execution_policy(ToolExecutionPolicy::new(
                CachePolicy::Pure,
                RetryPolicy::Never,
            ))
            .handler({
                let calls = Arc::clone(&calls);
                let started = Arc::clone(&started);
                let release = Arc::clone(&release);
                move |_| {
                    let calls = Arc::clone(&calls);
                    let started = Arc::clone(&started);
                    let release = Arc::clone(&release);
                    async move {
                        let invocation = calls.fetch_add(1, Ordering::SeqCst);
                        if invocation == 0 {
                            started.notify_one();
                            release.notified().await;
                        }
                        Ok(serde_json::json!({"invocation": invocation + 1}))
                    }
                }
            })
            .build()
            .unwrap();
        let executor = ToolExecutor::builder().enable_cache().build();
        executor.add_dyn_tool(Box::new(tool)).unwrap();

        let execution = {
            let executor = executor.clone();
            tokio::spawn(async move {
                executor
                    .execute("invalidate_fence", serde_json::json!({}))
                    .await
            })
        };
        started.notified().await;
        executor.invalidate_cache_for_tool("invalidate_fence");
        release.notify_one();
        assert!(execution.await.unwrap().unwrap().success);
        assert_eq!(executor.cache_stats().total_entries, 0);

        executor
            .execute("invalidate_fence", serde_json::json!({}))
            .await
            .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(executor.cache_stats().total_entries, 1);
    }

    #[tokio::test]
    async fn invalidating_one_tool_does_not_fence_another_tools_write_back() {
        let calls = Arc::new(AtomicUsize::new(0));
        let started = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let tool_a = FunctionTool::builder("epoch_a", "unrelated cache fixture")
            .execution_policy(ToolExecutionPolicy::new(
                CachePolicy::Pure,
                RetryPolicy::Never,
            ))
            .handler({
                let calls = Arc::clone(&calls);
                let started = Arc::clone(&started);
                let release = Arc::clone(&release);
                move |_| {
                    let calls = Arc::clone(&calls);
                    let started = Arc::clone(&started);
                    let release = Arc::clone(&release);
                    async move {
                        let invocation = calls.fetch_add(1, Ordering::SeqCst);
                        if invocation == 0 {
                            started.notify_one();
                            release.notified().await;
                        }
                        Ok(serde_json::json!({"tool": "a"}))
                    }
                }
            })
            .build()
            .unwrap();
        let tool_b = FunctionTool::builder("epoch_b", "invalidated cache fixture")
            .execution_policy(ToolExecutionPolicy::new(
                CachePolicy::Pure,
                RetryPolicy::Never,
            ))
            .handler(|_| async { Ok(serde_json::json!({"tool": "b"})) })
            .build()
            .unwrap();
        let executor = ToolExecutor::builder().enable_cache().build();
        executor.add_dyn_tool(Box::new(tool_a)).unwrap();
        executor.add_dyn_tool(Box::new(tool_b)).unwrap();

        let execution = {
            let executor = executor.clone();
            tokio::spawn(async move { executor.execute("epoch_a", serde_json::json!({})).await })
        };
        started.notified().await;
        executor.invalidate_cache_for_tool("epoch_b");
        release.notify_one();
        assert!(execution.await.unwrap().unwrap().success);

        let cached = executor
            .execute("epoch_a", serde_json::json!({}))
            .await
            .unwrap();
        assert!(cached.cache_hit);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    async fn assert_non_storing_cache_does_not_singleflight(
        executor: ToolExecutor,
        tool_name: &'static str,
    ) {
        let calls = Arc::new(AtomicUsize::new(0));
        let barrier = Arc::new(tokio::sync::Barrier::new(2));
        let tool = FunctionTool::builder(tool_name, "non-storing cache fixture")
            .execution_policy(ToolExecutionPolicy::new(
                CachePolicy::Pure,
                RetryPolicy::Never,
            ))
            .handler({
                let calls = Arc::clone(&calls);
                let barrier = Arc::clone(&barrier);
                move |_| {
                    let calls = Arc::clone(&calls);
                    let barrier = Arc::clone(&barrier);
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        barrier.wait().await;
                        Ok(serde_json::json!({"ok": true}))
                    }
                }
            })
            .build()
            .unwrap();
        executor.add_dyn_tool(Box::new(tool)).unwrap();

        let (first, second) = tokio::time::timeout(Duration::from_secs(1), async {
            tokio::join!(
                executor.execute(tool_name, serde_json::json!({})),
                executor.execute(tool_name, serde_json::json!({}))
            )
        })
        .await
        .expect("non-storing cache unexpectedly serialized the calls");
        assert!(first.unwrap().success);
        assert!(second.unwrap().success);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(executor.cache_stats().total_entries, 0);
        assert!(executor.cache_flights.is_empty());
    }

    #[tokio::test]
    async fn zero_capacity_and_zero_ttl_caches_do_not_singleflight() {
        assert_non_storing_cache_does_not_singleflight(
            ToolExecutor::builder()
                .enable_cache()
                .cache_max_size(0)
                .build(),
            "zero_capacity",
        )
        .await;
        assert_non_storing_cache_does_not_singleflight(
            ToolExecutor::builder()
                .enable_cache()
                .cache_ttl(Duration::ZERO)
                .build(),
            "zero_ttl",
        )
        .await;
    }
}
