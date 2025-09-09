//! Configuration management for ZAI Tools
//! 
//! This module provides comprehensive configuration management for tools,
//! registries, and executors with support for multiple configuration sources.

#[cfg(feature = "config-management")]
use config::{Config, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

/// Global configuration for ZAI Tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZaiToolsConfig {
    /// Registry configuration
    pub registry: RegistryConfig,
    /// Executor configuration
    pub executor: ExecutorConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Tool-specific configurations
    pub tools: HashMap<String, ToolConfig>,
}

/// Registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// Maximum number of tools allowed
    pub max_tools: Option<usize>,
    /// Enable concurrent access optimizations
    pub concurrent_access: bool,
    /// Cache tool metadata
    pub cache_metadata: bool,
    /// Auto-register built-in tools
    pub auto_register_builtin: bool,
    /// Tool discovery paths
    pub discovery_paths: Vec<String>,
}

/// Executor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorConfig {
    /// Default timeout for tool execution
    #[serde(with = "duration_serde")]
    pub default_timeout: Duration,
    /// Maximum number of retries
    pub max_retries: u32,
    /// Enable parallel execution
    pub parallel_execution: bool,
    /// Maximum concurrent executions
    pub max_concurrent: usize,
    /// Enable execution logging
    pub enable_logging: bool,
    /// Enable performance monitoring
    pub enable_monitoring: bool,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,
    /// Log format (json, pretty, compact)
    pub format: String,
    /// Enable structured logging
    pub structured: bool,
    /// Log file path (optional)
    pub file_path: Option<String>,
    /// Enable console logging
    pub console: bool,
}

/// Tool-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    /// Whether the tool is enabled
    pub enabled: bool,
    /// Tool-specific timeout
    #[serde(with = "duration_serde", skip_serializing_if = "Option::is_none")]
    pub timeout: Option<Duration>,
    /// Tool-specific retry count
    pub retries: Option<u32>,
    /// Tool-specific settings
    pub settings: HashMap<String, serde_json::Value>,
}

impl Default for ZaiToolsConfig {
    fn default() -> Self {
        Self {
            registry: RegistryConfig::default(),
            executor: ExecutorConfig::default(),
            logging: LoggingConfig::default(),
            tools: HashMap::new(),
        }
    }
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            max_tools: None,
            concurrent_access: true,
            cache_metadata: true,
            auto_register_builtin: true,
            discovery_paths: vec![],
        }
    }
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            default_timeout: Duration::from_secs(30),
            max_retries: 3,
            parallel_execution: true,
            max_concurrent: 10,
            enable_logging: true,
            enable_monitoring: false,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "pretty".to_string(),
            structured: false,
            file_path: None,
            console: true,
        }
    }
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            timeout: None,
            retries: None,
            settings: HashMap::new(),
        }
    }
}

/// Configuration builder for fluent API
pub struct ConfigBuilder {
    config: ZaiToolsConfig,
}

impl ConfigBuilder {
    /// Create a new configuration builder
    pub fn new() -> Self {
        Self {
            config: ZaiToolsConfig::default(),
        }
    }
    
    /// Set registry configuration
    pub fn registry(mut self, registry: RegistryConfig) -> Self {
        self.config.registry = registry;
        self
    }
    
    /// Set executor configuration
    pub fn executor(mut self, executor: ExecutorConfig) -> Self {
        self.config.executor = executor;
        self
    }
    
    /// Set logging configuration
    pub fn logging(mut self, logging: LoggingConfig) -> Self {
        self.config.logging = logging;
        self
    }
    
    /// Add tool configuration
    pub fn tool(mut self, name: impl Into<String>, config: ToolConfig) -> Self {
        self.config.tools.insert(name.into(), config);
        self
    }
    
    /// Build the configuration
    pub fn build(self) -> ZaiToolsConfig {
        self.config
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration loader with support for multiple sources
pub struct ConfigLoader {
    sources: Vec<ConfigSource>,
}

#[derive(Debug)]
enum ConfigSource {
    File(String),
    Environment(String),
    Defaults,
}

impl ConfigLoader {
    /// Create a new configuration loader
    pub fn new() -> Self {
        Self {
            sources: vec![ConfigSource::Defaults],
        }
    }
    
    /// Add a configuration file source
    pub fn add_file<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.sources.push(ConfigSource::File(
            path.as_ref().to_string_lossy().to_string()
        ));
        self
    }
    
    /// Add environment variable source
    pub fn add_env(mut self, prefix: impl Into<String>) -> Self {
        self.sources.push(ConfigSource::Environment(prefix.into()));
        self
    }
    
    /// Load configuration from all sources
    #[cfg(feature = "config-management")]
    pub fn load(self) -> Result<ZaiToolsConfig, ConfigError> {
        let mut config_builder = Config::builder();
        
        // Add sources in order
        for source in self.sources {
            match source {
                ConfigSource::File(path) => {
                    config_builder = config_builder.add_source(File::with_name(&path));
                }
                ConfigSource::Environment(prefix) => {
                    config_builder = config_builder.add_source(
                        Environment::with_prefix(&prefix).separator("__")
                    );
                }
                ConfigSource::Defaults => {
                    // Defaults are handled by the struct's Default implementation
                }
            }
        }
        
        let config = config_builder.build()?;
        config.try_deserialize()
    }
    
    /// Load configuration (fallback implementation without config crate)
    #[cfg(not(feature = "config-management"))]
    pub fn load(self) -> Result<ZaiToolsConfig, Box<dyn std::error::Error>> {
        // Return default configuration when config management is disabled
        Ok(ZaiToolsConfig::default())
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Serde helper for Duration serialization
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;
    
    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_secs())
    }
    
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

/// Configuration validation
impl ZaiToolsConfig {
    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        // Validate executor config
        if self.executor.max_concurrent == 0 {
            return Err("max_concurrent must be greater than 0".to_string());
        }
        
        if self.executor.default_timeout.as_secs() == 0 {
            return Err("default_timeout must be greater than 0".to_string());
        }
        
        // Validate logging config
        match self.logging.level.as_str() {
            "trace" | "debug" | "info" | "warn" | "error" => {}
            _ => return Err("invalid log level".to_string()),
        }
        
        match self.logging.format.as_str() {
            "json" | "pretty" | "compact" => {}
            _ => return Err("invalid log format".to_string()),
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = ZaiToolsConfig::default();
        assert!(config.validate().is_ok());
        assert_eq!(config.executor.default_timeout, Duration::from_secs(30));
        assert_eq!(config.executor.max_retries, 3);
        assert!(config.registry.auto_register_builtin);
    }
    
    #[test]
    fn test_config_builder() {
        let config = ConfigBuilder::new()
            .executor(ExecutorConfig {
                default_timeout: Duration::from_secs(60),
                max_retries: 5,
                ..Default::default()
            })
            .tool("calculator", ToolConfig {
                enabled: true,
                timeout: Some(Duration::from_secs(10)),
                ..Default::default()
            })
            .build();
        
        assert!(config.validate().is_ok());
        assert_eq!(config.executor.default_timeout, Duration::from_secs(60));
        assert_eq!(config.executor.max_retries, 5);
        assert!(config.tools.contains_key("calculator"));
    }
    
    #[test]
    fn test_config_validation() {
        let mut config = ZaiToolsConfig::default();
        config.executor.max_concurrent = 0;
        assert!(config.validate().is_err());
        
        config.executor.max_concurrent = 10;
        config.logging.level = "invalid".to_string();
        assert!(config.validate().is_err());
    }
}
