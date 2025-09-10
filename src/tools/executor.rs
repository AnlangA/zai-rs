//! Enhanced tool executor with type-safe builder pattern

use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use serde::{Deserialize, Serialize};
use tokio::time::timeout;
use tokio::task::JoinSet;

use crate::tools::core::{DynTool, Tool, ToolInput, ToolOutput, IntoDynTool};
use crate::tools::error::{ToolResult, error_context};

use crate::model::tools::{Function, Tools};
use crate::model::chat_base_response::ToolCallMessage;
use crate::model::chat_message_types::TextMessage;

/// Execution configuration with type-safe builder
#[derive(Debug, Clone)]
pub struct ExecutionConfig {
    pub timeout: Option<Duration>,
    pub max_retries: u32,
    pub retry_delay: Duration,
    pub validate_parameters: bool,
    pub enable_logging: bool,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            timeout: Some(Duration::from_secs(30)),
            max_retries: 0,
            retry_delay: Duration::from_millis(100),
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

    pub fn failure(
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
    tools: Arc<RwLock<HashMap<String, Box<dyn DynTool>>>>,
    config: ExecutionConfig,
}

impl std::fmt::Debug for ToolExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tools = self.tools.read().map(|m| m.len()).unwrap_or(0);
        f.debug_struct("ToolExecutor")
            .field("tool_count", &tools)
            .field("config", &self.config)
            .finish()
    }
}


/// Type alias for a dynamic function tool handler used when registering from external specs
pub type DynFunctionHandler = std::sync::Arc<
    dyn Fn(serde_json::Value) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = crate::tools::error::ToolResult<serde_json::Value>> + Send>
        > + Send + Sync
>;

impl ToolExecutor {
    /// Create a new executor with default config
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
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
        {
            let tools = self.tools.read().unwrap();
            if tools.contains_key(&name) {
                panic!("Tool '{}' is already registered", name);
            }
        }
        let mut tools = self.tools.write().unwrap();
        tools.insert(name, tool);
        self
    }

    /// Chain-friendly: try to add a typed tool (ignores error)
    pub fn try_add_tool<T, I, O>(&self, tool: T) -> &Self
    where
        T: Tool<I, O> + Clone + 'static,
        I: ToolInput,
        O: ToolOutput,
    {
        let name = tool.metadata().name.clone();
        let mut insert = true;
        if let Ok(guard) = self.tools.read() {
            if guard.contains_key(&name) { insert = false; }
        }
        if insert {
            if let Ok(mut guard) = self.tools.write() {
                guard.insert(name, tool.into_dyn_tool());
            }
        }
        self
    }

    /// Chain-friendly: try to add a dynamic tool (ignores error)
    pub fn try_add_dyn_tool(&self, tool: Box<dyn DynTool>) -> &Self {
        let name = tool.name().to_string();
        let mut insert = true;
        if let Ok(guard) = self.tools.read() {
            if guard.contains_key(&name) { insert = false; }
        }
        if insert {
            if let Ok(mut guard) = self.tools.write() {
                guard.insert(name, tool);
            }
        }
        self
    }

    /// Unregister a tool
    pub fn unregister(&self, name: &str) -> ToolResult<()> {
        let mut tools = self.tools.write().unwrap();
        if tools.remove(name).is_none() {
            return Err(error_context().tool_not_found());
        }
        Ok(())
    }

    /// Get input schema for a tool
    pub fn input_schema(&self, name: &str) -> Option<serde_json::Value> {
        let tools = self.tools.read().unwrap();
        tools.get(name).map(|t| t.input_schema())
    }

    /// Check if tool exists
    pub fn has_tool(&self, name: &str) -> bool {
        let tools = self.tools.read().unwrap();
        tools.contains_key(name)
    }

    /// List tool names
    pub fn tool_names(&self) -> Vec<String> {
        let tools = self.tools.read().unwrap();
        tools.keys().cloned().collect()
    }

    fn get_tool(&self, name: &str) -> Option<Box<dyn DynTool>> {
        let tools = self.tools.read().unwrap();
        tools.get(name).map(|t| t.clone_box())
    }

    /// Execute a tool with detailed result
    pub async fn execute(&self, tool_name: &str, input: serde_json::Value) -> ToolResult<ExecutionResult> {
        let start_time = Instant::now();
        let mut retries = 0;

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
                    if retries >= self.config.max_retries {
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

                    tokio::time::sleep(self.config.retry_delay).await;
                }
            }
        }
    }

    /// Execute a tool and return only the result
    pub async fn execute_simple(&self, tool_name: &str, input: serde_json::Value) -> ToolResult<serde_json::Value> {
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
        handlers: &std::collections::HashMap<String, DynFunctionHandler>,
        strict: bool,
    ) -> ToolResult<Vec<String>> {
        use std::fs;
        use serde_json::Value;
        let dir = dir.as_ref();
        let mut added = Vec::new();
        let read_dir = fs::read_dir(dir).map_err(|e| error_context().invalid_parameters(format!("Failed to read dir {}: {}", dir.display(), e)))?;
        for entry in read_dir {
            let entry = match entry { Ok(e) => e, Err(e) => return Err(error_context().invalid_parameters(format!("Dir entry error: {}", e))) };
            let path = entry.path();
            if !path.is_file() { continue; }
            if path.extension().and_then(|s| s.to_str()) != Some("json") { continue; }
            let content = fs::read_to_string(&path).map_err(|e| error_context().invalid_parameters(format!("Failed to read {}: {}", path.display(), e)))?;
            let spec: Value = serde_json::from_str(&content).map_err(|e| error_context().invalid_parameters(format!("Invalid JSON in {}: {}", path.display(), e)))?;

            // Extract name/description/parameters from spec
            let (name, description, parameters) = {
                let obj = spec.as_object().ok_or_else(|| error_context().invalid_parameters(format!("Spec must be object: {}", path.display())))?;
                if obj.get("type").and_then(|v| v.as_str()) == Some("function") {
                    let f = obj.get("function").and_then(|v| v.as_object()).ok_or_else(|| error_context().invalid_parameters(format!("Missing 'function' object in {}", path.display())))?;
                    let name = f.get("name").and_then(|v| v.as_str()).ok_or_else(|| error_context().invalid_parameters(format!("Missing function.name in {}", path.display())))?.to_string();
                    let desc = f.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let params = f.get("parameters").cloned();
                    (name, desc, params)
                } else {
                    let name = obj.get("name").and_then(|v| v.as_str()).ok_or_else(|| error_context().invalid_parameters(format!("Missing name in {}", path.display())))?.to_string();
                    let desc = obj.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let params = obj.get("parameters").cloned();
                    (name, desc, params)
                }
            };

            let handler = match handlers.get(&name) {
                Some(h) => h.clone(),
                None => {
                    if strict {
                        return Err(error_context().invalid_parameters(format!("No handler registered for function '{}' (file {})", name, path.display())));
                    } else {
                        // skip silently
                        continue;
                    }
                }
            };

            // Build FunctionTool via existing builder path (will auto-complete schema defaults)
            let mut builder = crate::tools::core::FunctionTool::builder(name.clone(), description);
            if let Some(p) = parameters { builder = builder.schema(p); }
            let tool = builder.handler(move |args| {
                let h = handler.clone();
                h(args)
            }).build()?;

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
    /// - Vec<TextMessage> ready to be appended to ChatCompletion as tool messages.
    pub async fn execute_tool_calls_parallel(&self, calls: &[ToolCallMessage]) -> Vec<TextMessage> {
        let mut set = JoinSet::new();
        for tc in calls {
            let id_opt = tc.id().map(|s| s.to_string());
            let func_opt = tc.function();
            let this = self.clone();
            if let Some(func) = func_opt {
                let name = func.name().unwrap_or("").to_string();
                let args_str = func.arguments().unwrap_or("{}");
                let args_json: serde_json::Value = serde_json::from_str(args_str)
                    .unwrap_or_else(|_| serde_json::json!({ "_raw": args_str }));
                set.spawn(async move {
                    let content_json = match this.execute_simple(&name, args_json).await {
                        Ok(v) => v,
                        Err(err) => serde_json::json!({ "error": { "type": "execution_failed", "message": err.to_string() } }),
                    };
                    let s = serde_json::to_string(&content_json).unwrap_or_else(|_| "{}".to_string());
                    if let Some(id) = id_opt { TextMessage::tool_with_id(s, id) } else { TextMessage::tool(s) }
                });
            } else {
                let id_copy = id_opt.clone();
                set.spawn(async move {
                    let s = serde_json::json!({ "error": { "type": "missing_function", "message": "tool_call.function is missing" } }).to_string();
                    if let Some(id) = id_copy { TextMessage::tool_with_id(s, id) } else { TextMessage::tool(s) }
                });
            }
        }
        let mut messages = Vec::with_capacity(calls.len());
        while let Some(res) = set.join_next().await {
            if let Ok(msg) = res { messages.push(msg); }
        }
        messages
    }



    /// Export a single registered tool as Tools::Function (for LLM function calling)
    pub fn export_tool_as_function(&self, name: &str) -> Option<Tools> {
        let tools = self.tools.read().ok()?;
        let tool = tools.get(name)?;
        let meta = tool.metadata();
        let schema = tool.input_schema();
        let func = Function::new(meta.name.clone(), meta.description.clone(), schema);
        Some(Tools::Function { function: func })
    }

    /// Export all registered tools as a Vec<Tools::Function>
    pub fn export_all_tools_as_functions(&self) -> Vec<Tools> {
        let tools = match self.tools.read() { Ok(t) => t, Err(_) => return Vec::new() };
        tools
            .values()
            .map(|tool| {
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
        F: FnMut(&crate::tools::core::ToolMetadata) -> bool,
    {
        let tools = match self.tools.read() { Ok(t) => t, Err(_) => return Vec::new() };
        tools
            .values()
            .filter(|tool| filter(tool.metadata()))
            .map(|tool| {
                let meta = tool.metadata();
                let schema = tool.input_schema();
                let func = Function::new(meta.name.clone(), meta.description.clone(), schema);
                Tools::Function { function: func }
            })
            .collect()
    }




    async fn execute_once(&self, tool_name: &str, input: &serde_json::Value) -> ToolResult<serde_json::Value> {
        let tool = self
            .get_tool(tool_name)
            .ok_or_else(|| error_context().with_tool(tool_name).tool_not_found())?;
        let execution_future = tool.execute_json(input.clone());

        match self.config.timeout {
            Some(timeout_duration) => {
                match timeout(timeout_duration, execution_future).await {
                    Ok(result) => result,
                    Err(_) => Err(error_context()
                        .with_tool(tool_name)
                        .timeout_error(timeout_duration)),
                }
            }
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
        self.config.max_retries = retries;
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
            tools: Arc::new(RwLock::new(HashMap::new())),
            config: self.config,
        }
    }
}

