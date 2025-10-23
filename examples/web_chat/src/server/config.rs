//! Application configuration management

use serde::{Deserialize, Serialize};
use std::env;

/// Application configuration loaded from environment variables
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Server port
    pub port: u16,
    
    /// API key for Zhipu AI
    pub api_key: String,
    
    /// CORS allowed origins
    pub cors_origins: Vec<String>,
    
    /// Maximum session duration in seconds
    pub session_timeout: u64,
    
    /// Maximum messages per session
    pub max_messages_per_session: usize,
    
    /// Request timeout in seconds
    pub request_timeout: u64,
    
    /// Enable request logging
    pub enable_logging: bool,
    
    /// Static file directory
    pub static_dir: String,
    
    /// Maximum upload file size in bytes
    pub max_file_size: usize,
    
    /// Enable file uploads
    pub enable_file_upload: bool,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Config {
            port: env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .map_err(|_| ConfigError::InvalidPort)?,
                
            api_key: env::var("ZHIPU_API_KEY")
                .map_err(|_| ConfigError::MissingApiKey)?,
                
            cors_origins: env::var("CORS_ORIGINS")
                .unwrap_or_else(|_| "http://localhost:3000,http://127.0.0.1:3000".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
                
            session_timeout: env::var("SESSION_TIMEOUT")
                .unwrap_or_else(|_| "3600".to_string()) // 1 hour
                .parse()
                .map_err(|_| ConfigError::InvalidSessionTimeout)?,
                
            max_messages_per_session: env::var("MAX_MESSAGES_PER_SESSION")
                .unwrap_or_else(|_| "1000".to_string())
                .parse()
                .map_err(|_| ConfigError::InvalidMaxMessages)?,
                
            request_timeout: env::var("REQUEST_TIMEOUT")
                .unwrap_or_else(|_| "30".to_string()) // 30 seconds
                .parse()
                .map_err(|_| ConfigError::InvalidRequestTimeout)?,
                
            enable_logging: env::var("ENABLE_LOGGING")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .map_err(|_| ConfigError::InvalidLoggingFlag)?,
                
            static_dir: env::var("STATIC_DIR")
                .unwrap_or_else(|_| "examples/web_chat/static".to_string()),
                
            max_file_size: env::var("MAX_FILE_SIZE")
                .unwrap_or_else(|_| "10485760".to_string()) // 10MB
                .parse()
                .map_err(|_| ConfigError::InvalidMaxFileSize)?,
                
            enable_file_upload: env::var("ENABLE_FILE_UPLOAD")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .map_err(|_| ConfigError::InvalidFileUploadFlag)?,
        })
    }
    
    /// Validate configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.port == 0 || self.port > 65535 {
            return Err(ConfigError::InvalidPort);
        }
        
        if self.api_key.is_empty() {
            return Err(ConfigError::MissingApiKey);
        }
        
        if self.session_timeout == 0 {
            return Err(ConfigError::InvalidSessionTimeout);
        }
        
        if self.max_messages_per_session == 0 {
            return Err(ConfigError::InvalidMaxMessages);
        }
        
        if self.request_timeout == 0 {
            return Err(ConfigError::InvalidRequestTimeout);
        }
        
        if self.max_file_size == 0 {
            return Err(ConfigError::InvalidMaxFileSize);
        }
        
        Ok(())
    }
}

/// Configuration-related errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Missing ZHIPU_API_KEY environment variable")]
    MissingApiKey,
    
    #[error("Invalid port number")]
    InvalidPort,
    
    #[error("Invalid session timeout")]
    InvalidSessionTimeout,
    
    #[error("Invalid max messages per session")]
    InvalidMaxMessages,
    
    #[error("Invalid request timeout")]
    InvalidRequestTimeout,
    
    #[error("Invalid logging flag")]
    InvalidLoggingFlag,
    
    #[error("Invalid max file size")]
    InvalidMaxFileSize,
    
    #[error("Invalid file upload flag")]
    InvalidFileUploadFlag,
}