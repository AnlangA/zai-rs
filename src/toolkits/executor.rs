//! Tool registry and executor with caching, retries, and bounded concurrency.

use std::{
    panic::AssertUnwindSafe,
    sync::Arc,
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

use dashmap::{DashMap, mapref::entry::Entry};
use futures_util::{FutureExt, StreamExt};
use tokio::time::timeout;
use tracing::warn;

use super::{
    cache::{CacheKey, ToolCallCache},
    core::{ToolHandler, validate_tool_name},
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
    config: ExecutionConfig,
    cache: ToolCallCache,
}

struct RegisteredTool {
    tool: Arc<dyn DynTool>,
    generation: u64,
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
            config: ExecutionConfig::default(),
            // Tool purity is unknown at registration time, so caching is an
            // explicit opt-in.
            cache: ToolCallCache::new().with_enabled(false),
        }
    }

    /// Create an executor builder for fluent API
    pub fn builder() -> ExecutorBuilder {
        ExecutorBuilder::new()
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
    pub fn cache_stats(&self) -> super::CacheStats {
        self.cache.stats()
    }

    /// Chain-friendly: add a dynamic tool, returns error if already registered
    pub fn add_dyn_tool(&self, tool: Box<dyn DynTool>) -> ToolResult<&Self> {
        let name = tool.name().to_string();
        validate_tool_name(&name)?;
        match self.tools.entry(name.clone()) {
            Entry::Occupied(_) => Err(ToolError::RegistrationError {
                message: format!("Tool '{name}' is already registered").into(),
            }),
            Entry::Vacant(entry) => {
                let generation = self.next_generation.fetch_add(1, Ordering::Relaxed);
                entry.insert(RegisteredTool {
                    tool: Arc::from(tool),
                    generation,
                });
                // Remove entries from a prior registration with this name.
                self.cache.invalidate_tool(&name);
                Ok(self)
            },
        }
    }

    /// Unregister a tool
    pub fn unregister(&self, name: &str) -> ToolResult<()> {
        if self.tools.remove(name).is_none() {
            return Err(error_context().with_tool(name).tool_not_found());
        }
        self.cache.invalidate_tool(name);
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

    fn get_tool(&self, name: &str) -> Option<(Arc<dyn DynTool>, u64)> {
        self.tools
            .get(name)
            .map(|entry| (Arc::clone(&entry.tool), entry.generation))
    }

    fn cache_if_still_registered(
        &self,
        tool_name: &str,
        generation: u64,
        key: CacheKey,
        result: serde_json::Value,
    ) {
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

        let Some((tool, generation)) = self.get_tool(tool_name) else {
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

        // Registration generation prevents an in-flight call from repopulating
        // a cache entry that a later tool with the same name could consume.
        let cache_key = if self.cache.enabled() {
            Some(CacheKey::for_generation(
                tool_name.to_string(),
                input.clone(),
                generation,
            ))
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
            .with_cache_hit());
        }

        loop {
            match self.execute_once(&tool, tool_name, &input).await {
                Ok(result) => {
                    let duration = start_time.elapsed();
                    if let Some(key) = cache_key {
                        self.cache_if_still_registered(tool_name, generation, key, result.clone());
                    }

                    return Ok(ExecutionResult::success(
                        tool_name.to_string(),
                        result,
                        duration,
                        retries,
                    ));
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

            let handler = match handlers.get(&name) {
                Some(handler) => Arc::clone(handler),
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

            let mut builder =
                crate::toolkits::core::FunctionTool::builder(name.clone(), description);
            if let Some(p) = parameters {
                builder = builder.schema(p);
            }
            let tool = builder
                .handler(move |args| {
                    let handler = Arc::clone(&handler);
                    handler(args)
                })
                .build()?;

            self.add_dyn_tool(Box::new(tool))?;
            added.push(name);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    use crate::toolkits::core::FunctionTool;

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
}
