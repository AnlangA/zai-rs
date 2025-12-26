//! Enhanced tool executor with type-safe builder pattern

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::task::JoinSet;
use tokio::time::timeout;

use crate::toolkits::core::DynTool;
use crate::toolkits::error::{ToolResult, error_context};

use crate::model::chat_base_response::ToolCallMessage;
use crate::model::chat_message_types::TextMessage;
use crate::model::tools::{Function, Tools};

/// Type alias for the complex handler type to reduce complexity warnings
type ToolHandler = std::sync::Arc<
    dyn Fn(
            serde_json::Value,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = crate::toolkits::error::ToolResult<serde_json::Value>,
                    > + Send,
            >,
        > + Send
        + Sync,
>;

/// Enhanced retry configuration with exponential backoff
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
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
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::ZERO;
        }

        let delay_ms = self.initial_delay.as_millis() as f64
            * self.backoff_multiplier.powi((attempt - 1) as i32);
        let delay_ms = delay_ms.min(self.max_delay.as_millis() as f64) as u64;

        Duration::from_millis(delay_ms)
    }
}

/// Execution configuration with type-safe builder
#[derive(Debug, Clone)]
pub struct ExecutionConfig {
    pub timeout: Option<Duration>,
    pub retry_config: RetryConfig,
    pub validate_parameters: bool,
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
    pub tool_name: String,
    pub result: serde_json::Value,
    pub duration: Duration,
    pub success: bool,
    pub error: Option<String>,
    pub retries: u32,
    pub timestamp: std::time::SystemTime,
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

impl ExecutionResult {
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

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Enhanced tool executor with built-in registry and fluent API
#[derive(Clone)]
pub struct ToolExecutor {
    tools: Arc<DashMap<String, Box<dyn DynTool>>>,
    config: ExecutionConfig,
}

impl std::fmt::Debug for ToolExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tool_count = self.tools.len();
        f.debug_struct("ToolExecutor")
            .field("tool_count", &tool_count)
            .field("config", &self.config)
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
        }
    }

    /// Create an executor builder for fluent API
    pub fn builder() -> ExecutorBuilder {
        ExecutorBuilder::new()
    }

    /// Chain-friendly: add a dynamic tool (panics on error)
    pub fn add_dyn_tool(&self, tool: Box<dyn DynTool>) -> &Self {
        let name = tool.name().to_string();
        if self.tools.contains_key(&name) {
            panic!("Tool '{}' is already registered", name);
        }
        self.tools.insert(name, tool);
        self
    }

    /// Chain-friendly: try to add a dynamic tool (ignores error)
    pub fn try_add_dyn_tool(&self, tool: Box<dyn DynTool>) -> &Self {
        let name = tool.name().to_string();
        self.tools.entry(name).or_insert(tool);
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

    fn get_tool(&self, name: &str) -> Option<Box<dyn DynTool>> {
        self.tools.get(name).map(|t| t.clone_box())
    }

    /// Execute a tool with detailed result and exponential backoff
    pub async fn execute(
        &self,
        tool_name: &str,
        input: serde_json::Value,
    ) -> ToolResult<ExecutionResult> {
        let start_time = Instant::now();
        let mut retries = 0;
        let retry_config = &self.config.retry_config;

        loop {
            match self.execute_once(tool_name, &input).await {
                Ok(result) => {
                    let duration = start_time.elapsed();
                    return Ok(ExecutionResult::success(
                        tool_name.to_string(),
                        result,
                        duration,
                        retries,
                    ));
                }
                Err(error) => {
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

                    if self.config.enable_logging {
                        eprintln!("Tool execution failed (attempt {}): {}", retries, error);
                    }

                    // Use exponential backoff
                    let delay = retry_config.calculate_delay(retries);
                    tokio::time::sleep(delay).await;
                }
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

    /// Bulk load function specs from a directory of .json files and register them with handlers.
    ///
    /// - Each file should contain either of the following shapes:
    ///   1) {"name":..., "description":..., "parameters": {...}}
    ///   2) {"type":"function", "function": {"name":..., "description":..., "parameters": {...}}}
    /// - `handlers` maps function `name` -> handler closure
    /// - `strict`: when true, missing handler for any spec will return error; when false, specs without handlers are skipped
    ///
    /// Returns the list of function names successfully registered.
    pub fn add_functions_from_dir_with_registry(
        &self,
        dir: impl AsRef<std::path::Path>,
        handlers: &std::collections::HashMap<String, ToolHandler>,
        strict: bool,
    ) -> ToolResult<Vec<String>> {
        use serde_json::Value;
        use std::fs;
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
                }
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
                }
            };

            // Build FunctionTool via existing builder path (will auto-complete schema defaults)
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

            self.add_dyn_tool(Box::new(tool));
            added.push(name);
        }
        Ok(added)
    }

    /// Execute LLM tool_calls in parallel and return `TextMessage::tool` messages.
    ///
    /// Behavior:
    /// - Parses each ToolCallMessage's function.arguments (stringified JSON supported)
    /// - Runs all tools concurrently using this executor
    /// - Captures errors per-call and encodes them as JSON: { "error": { "type": "...", "message": "..." } }
    /// - Preserves tool_call `id` by emitting TextMessage::tool_with_id when present
    ///
    /// Returns:
    /// - `Vec<TextMessage>` ready to be appended to ChatCompletion as tool messages.
    async fn execute_single_tool_call(&self, tc: &ToolCallMessage) -> TextMessage {
        let id_opt = tc.id().map(|s| s.to_string());
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

    pub async fn execute_tool_calls_parallel(&self, calls: &[ToolCallMessage]) -> Vec<TextMessage> {
        let mut set = JoinSet::new();

        // Clone the calls to avoid borrowing issues
        let calls_vec = calls.to_vec();
        for tc in calls_vec {
            let this = self.clone();
            set.spawn(async move { this.execute_single_tool_call(&tc).await });
        }

        let mut messages = Vec::with_capacity(calls.len());
        while let Some(res) = set.join_next().await {
            if let Ok(msg) = res {
                messages.push(msg);
            }
        }
        messages
    }

    /// Execute LLM tool_calls in parallel with result ordering preserved
    ///
    /// This method guarantees that results are returned in the same order as the input calls,
    /// which is important for maintaining conversation context in LLM interactions.
    ///
    /// Behavior:
    /// - Parses each ToolCallMessage's function.arguments (stringified JSON supported)
    /// - Runs all tools concurrently using this executor
    /// - Preserves the original order of tool calls in results
    /// - Captures errors per-call and encodes them as JSON
    /// - Preserves tool_call `id` by emitting TextMessage::tool_with_id when present
    ///
    /// Returns:
    /// - Vec<TextMessage> in the same order as input calls, ready for ChatCompletion
    pub async fn execute_tool_calls_ordered(&self, calls: &[ToolCallMessage]) -> Vec<TextMessage> {
        use futures::future::join_all;

        let calls_vec = calls.to_vec();
        let futures: Vec<_> = calls_vec
            .into_iter()
            .map(|tc| {
                let this = self.clone();
                async move { this.execute_single_tool_call(&tc).await }
            })
            .collect();

        join_all(futures).await
    }

    /// Export a single registered tool as Tools::Function (for LLM function calling)
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

    async fn execute_once(
        &self,
        tool_name: &str,
        input: &serde_json::Value,
    ) -> ToolResult<serde_json::Value> {
        let tool = self
            .get_tool(tool_name)
            .ok_or_else(|| error_context().with_tool(tool_name).tool_not_found())?;
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

    /// Get the config
    pub fn config(&self) -> &ExecutionConfig {
        &self.config
    }
}

/// Builder for creating tool executors with fluent API
pub struct ExecutorBuilder {
    config: ExecutionConfig,
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

    /// Build the final executor
    pub fn build(self) -> ToolExecutor {
        ToolExecutor {
            tools: Arc::new(DashMap::new()),
            config: self.config,
        }
    }
}
