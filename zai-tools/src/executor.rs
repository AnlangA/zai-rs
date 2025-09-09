//! Enhanced tool executor with type-safe builder pattern

use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use tokio::time::timeout;

use crate::registry::ToolRegistry;
use crate::error::{ToolResult, error_context};

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

/// Type-safe builder for execution configuration
pub struct ConfigBuilder<State = Initial> {
    config: ExecutionConfig,
    _state: std::marker::PhantomData<State>,
}

/// Builder states
pub struct Initial;
pub struct WithTimeout;
pub struct WithRetries;
pub struct Configured;

impl ConfigBuilder<Initial> {
    pub fn new() -> Self {
        Self {
            config: ExecutionConfig::default(),
            _state: std::marker::PhantomData,
        }
    }
    
    pub fn timeout(mut self, timeout: Duration) -> ConfigBuilder<WithTimeout> {
        self.config.timeout = Some(timeout);
        ConfigBuilder {
            config: self.config,
            _state: std::marker::PhantomData,
        }
    }
    
    pub fn no_timeout(mut self) -> ConfigBuilder<WithTimeout> {
        self.config.timeout = None;
        ConfigBuilder {
            config: self.config,
            _state: std::marker::PhantomData,
        }
    }
}

impl ConfigBuilder<WithTimeout> {
    pub fn retries(mut self, max_retries: u32, delay: Duration) -> ConfigBuilder<WithRetries> {
        self.config.max_retries = max_retries;
        self.config.retry_delay = delay;
        ConfigBuilder {
            config: self.config,
            _state: std::marker::PhantomData,
        }
    }
    
    pub fn no_retries(self) -> ConfigBuilder<WithRetries> {
        ConfigBuilder {
            config: self.config,
            _state: std::marker::PhantomData,
        }
    }
}

impl ConfigBuilder<WithRetries> {
    pub fn validation(mut self, enabled: bool) -> ConfigBuilder<Configured> {
        self.config.validate_parameters = enabled;
        ConfigBuilder {
            config: self.config,
            _state: std::marker::PhantomData,
        }
    }
    
    pub fn logging(mut self, enabled: bool) -> ConfigBuilder<Configured> {
        self.config.enable_logging = enabled;
        ConfigBuilder {
            config: self.config,
            _state: std::marker::PhantomData,
        }
    }
}

impl ConfigBuilder<Configured> {
    pub fn build(self) -> ExecutionConfig {
        self.config
    }
}

// Allow building from any state for convenience
impl<State> ConfigBuilder<State> {
    pub fn build_unchecked(self) -> ExecutionConfig {
        self.config
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

/// Enhanced tool executor with fluent API
#[derive(Debug, Clone)]
pub struct ToolExecutor {
    registry: ToolRegistry,
    config: ExecutionConfig,
}

impl ToolExecutor {
    /// Create a new executor with default config
    pub fn new(registry: ToolRegistry) -> Self {
        Self {
            registry,
            config: ExecutionConfig::default(),
        }
    }

    /// Create an executor builder for fluent API
    pub fn builder(registry: ToolRegistry) -> ExecutorBuilder {
        ExecutorBuilder::new(registry)
    }
    
    /// Create executor with custom config
    pub fn with_config(registry: ToolRegistry, config: ExecutionConfig) -> Self {
        Self { registry, config }
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
    
    /// Execute multiple tools in parallel
    pub async fn execute_parallel(&self, requests: Vec<(String, serde_json::Value)>) -> Vec<ToolResult<ExecutionResult>> {
        let futures = requests.into_iter().map(|(tool_name, input)| {
            let executor = self.clone();
            async move {
                executor.execute(&tool_name, input).await
            }
        });
        
        futures::future::join_all(futures).await
    }
    
    /// Execute with a specific timeout
    pub async fn execute_with_timeout(
        &self,
        tool_name: &str,
        input: serde_json::Value,
        timeout_duration: Duration,
    ) -> ToolResult<ExecutionResult> {
        match timeout(timeout_duration, self.execute(tool_name, input)).await {
            Ok(result) => result,
            Err(_) => Ok(ExecutionResult::failure(
                tool_name.to_string(),
                format!("Execution timed out after {:?}", timeout_duration),
                timeout_duration,
                0,
            )),
        }
    }
    
    async fn execute_once(&self, tool_name: &str, input: &serde_json::Value) -> ToolResult<serde_json::Value> {
        let execution_future = self.registry.execute(tool_name, input.clone());
        
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
    
    /// Get the registry
    pub fn registry(&self) -> &ToolRegistry {
        &self.registry
    }
    
    /// Get the config
    pub fn config(&self) -> &ExecutionConfig {
        &self.config
    }
}



/// Builder for creating tool executors with fluent API
pub struct ExecutorBuilder {
    registry: ToolRegistry,
    config: ExecutionConfig,
}

impl ExecutorBuilder {
    /// Create a new executor builder
    pub fn new(registry: ToolRegistry) -> Self {
        Self {
            registry,
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
            registry: self.registry,
            config: self.config,
        }
    }
}
