//! Session-related data models

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Session information response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Session ID
    pub id: String,
    
    /// Session creation time
    pub created_at: DateTime<Utc>,
    
    /// Last activity time
    pub last_activity: DateTime<Utc>,
    
    /// Number of messages in session
    pub message_count: usize,
    
    /// Session metadata
    pub metadata: SessionMetadata,
    
    /// Session status
    pub status: SessionStatus,
}

/// Session metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    /// User agent string
    pub user_agent: Option<String>,
    
    /// Client IP address
    pub client_ip: Option<String>,
    
    /// Preferred language
    pub language: Option<String>,
    
    /// Think mode enabled
    pub think_mode: bool,
    
    /// Total tokens used
    pub total_tokens: u64,
    
    /// Number of requests made
    pub request_count: u64,
    
    /// Average response time in milliseconds
    pub avg_response_time_ms: Option<u64>,
    
    /// Last model used
    pub last_model: Option<String>,
}

/// Session status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    /// Active session
    Active,
    
    /// Session has expired
    Expired,
    
    /// Session has been closed
    Closed,
}

/// Session creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    /// Optional session ID (if not provided, one will be generated)
    pub session_id: Option<String>,
    
    /// Session metadata
    pub metadata: Option<SessionMetadataInput>,
}

/// Session metadata input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadataInput {
    /// User agent string
    pub user_agent: Option<String>,
    
    /// Preferred language
    pub language: Option<String>,
    
    /// Enable think mode by default
    pub think_mode: Option<bool>,
}

/// Session creation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionResponse {
    /// Created session ID
    pub session_id: String,
    
    /// Session information
    pub session: SessionInfo,
    
    /// Welcome message
    pub welcome_message: String,
}

/// Session update request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSessionRequest {
    /// Session metadata updates
    pub metadata: Option<SessionMetadataInput>,
    
    /// New status
    pub status: Option<SessionStatus>,
}

/// Session list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionListResponse {
    /// Total number of sessions
    pub total: usize,
    
    /// Sessions
    pub sessions: Vec<SessionInfo>,
    
    /// Pagination information
    pub pagination: PaginationInfo,
}

/// Pagination information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationInfo {
    /// Current page
    pub page: u32,
    
    /// Items per page
    pub per_page: u32,
    
    /// Total pages
    pub total_pages: u32,
    
    /// Has next page
    pub has_next: bool,
    
    /// Has previous page
    pub has_prev: bool,
}

/// Session statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStats {
    /// Total number of sessions
    pub total_sessions: usize,
    
    /// Number of active sessions
    pub active_sessions: usize,
    
    /// Number of expired sessions
    pub expired_sessions: usize,
    
    /// Average session duration in seconds
    pub avg_session_duration_secs: Option<f64>,
    
    /// Total messages across all sessions
    pub total_messages: usize,
    
    /// Average messages per session
    pub avg_messages_per_session: f64,
    
    /// Total tokens used across all sessions
    pub total_tokens: u64,
    
    /// Average tokens per session
    pub avg_tokens_per_session: f64,
}

/// Session export format
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    /// JSON format
    Json,
    
    /// Markdown format
    Markdown,
    
    /// Plain text format
    Text,
    
    /// HTML format
    Html,
}

/// Session export request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportSessionRequest {
    /// Export format
    pub format: ExportFormat,
    
    /// Include metadata
    pub include_metadata: bool,
    
    /// Include timestamps
    pub include_timestamps: bool,
}

/// Session export response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportSessionResponse {
    /// Exported content
    pub content: String,
    
    /// Content type
    pub content_type: String,
    
    /// Filename suggestion
    pub filename: String,
    
    /// Export metadata
    pub metadata: ExportMetadata,
}

/// Export metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportMetadata {
    /// Export timestamp
    pub exported_at: DateTime<Utc>,
    
    /// Export format
    pub format: ExportFormat,
    
    /// Number of messages exported
    pub message_count: usize,
    
    /// Session ID
    pub session_id: String,
    
    /// Export version
    pub version: String,
}

impl SessionInfo {
    /// Create session info from internal session data
    pub fn from_session(session: &crate::server::state::ChatSession) -> Self {
        Self {
            id: session.id.clone(),
            created_at: session.created_at,
            last_activity: session.last_activity,
            message_count: session.messages.len(),
            metadata: SessionMetadata {
                user_agent: session.metadata.user_agent.clone(),
                client_ip: session.metadata.client_ip.clone(),
                language: session.metadata.language.clone(),
                think_mode: session.metadata.think_mode,
                total_tokens: session.metadata.total_tokens,
                request_count: session.metadata.request_count,
                avg_response_time_ms: None, // Calculate if needed
                last_model: None, // Set if needed
            },
            status: SessionStatus::Active, // Determine based on timeout
        }
    }
    
    /// Calculate session duration in seconds
    pub fn duration_secs(&self) -> i64 {
        self.last_activity.signed_duration_since(self.created_at).num_seconds()
    }
    
    /// Check if session is expired based on timeout
    pub fn is_expired(&self, timeout_secs: u64) -> bool {
        let now = Utc::now();
        now.signed_duration_since(self.last_activity).num_seconds() > timeout_secs as i64
    }
}

impl SessionMetadata {
    /// Create default metadata
    pub fn default() -> Self {
        Self {
            user_agent: None,
            client_ip: None,
            language: None,
            think_mode: false,
            total_tokens: 0,
            request_count: 0,
            avg_response_time_ms: None,
            last_model: None,
        }
    }
    
    /// Update with new request information
    pub fn update_with_request(&mut self, model: Option<String>, tokens: u64, response_time_ms: u64) {
        self.request_count += 1;
        self.total_tokens += tokens;
        self.last_model = model;
        
        // Update average response time
        if let Some(avg) = self.avg_response_time_ms {
            self.avg_response_time_ms = Some((avg * (self.request_count - 1) + response_time_ms) / self.request_count);
        } else {
            self.avg_response_time_ms = Some(response_time_ms);
        }
    }
}

impl CreateSessionRequest {
    /// Generate a new session ID if not provided
    pub fn get_or_generate_session_id(&self) -> String {
        self.session_id.clone().unwrap_or_else(|| Uuid::new_v4().to_string())
    }
}

impl ExportSessionResponse {
    /// Create export response for a session
    pub fn create(
        session: &crate::server::state::ChatSession,
        format: ExportFormat,
        include_metadata: bool,
        include_timestamps: bool,
    ) -> Self {
        let content = match format {
            ExportFormat::Json => export_as_json(session, include_metadata, include_timestamps),
            ExportFormat::Markdown => export_as_markdown(session, include_metadata, include_timestamps),
            ExportFormat::Text => export_as_text(session, include_metadata, include_timestamps),
            ExportFormat::Html => export_as_html(session, include_metadata, include_timestamps),
        };
        
        let content_type = match format {
            ExportFormat::Json => "application/json",
            ExportFormat::Markdown => "text/markdown",
            ExportFormat::Text => "text/plain",
            ExportFormat::Html => "text/html",
        }.to_string();
        
        let filename = format!("chat_session_{}.{}", session.id, match format {
            ExportFormat::Json => "json",
            ExportFormat::Markdown => "md",
            ExportFormat::Text => "txt",
            ExportFormat::Html => "html",
        });
        
        Self {
            content,
            content_type,
            filename,
            metadata: ExportMetadata {
                exported_at: Utc::now(),
                format,
                message_count: session.messages.len(),
                session_id: session.id.clone(),
                version: "1.0.0".to_string(),
            },
        }
    }
}

// Export format implementations
fn export_as_json(
    session: &crate::server::state::ChatSession,
    include_metadata: bool,
    include_timestamps: bool,
) -> String {
    #[derive(Serialize)]
    struct ExportData<'a> {
        session_id: &'a str,
        created_at: DateTime<Utc>,
        messages: Vec<ExportMessage<'a>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<&'a crate::server::state::SessionMetadata>,
    }
    
    #[derive(Serialize)]
    struct ExportMessage<'a> {
        role: &'a str,
        content: &'a serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        timestamp: Option<DateTime<Utc>>,
    }
    
    let messages: Vec<ExportMessage> = session.messages.iter().map(|msg| ExportMessage {
        role: &msg.role,
        content: &msg.content,
        timestamp: if include_timestamps { Some(session.created_at) } else { None },
    }).collect();
    
    let data = ExportData {
        session_id: &session.id,
        created_at: session.created_at,
        messages,
        metadata: if include_metadata { Some(&session.metadata) } else { None },
    };
    
    serde_json::to_string_pretty(&data).unwrap_or_default()
}

fn export_as_markdown(
    session: &crate::server::state::ChatSession,
    include_metadata: bool,
    include_timestamps: bool,
) -> String {
    let mut content = String::new();
    
    // Header
    content.push_str(&format!("# Chat Session: {}\n\n", session.id));
    content.push_str(&format!("**Created:** {}\n", session.created_at.format("%Y-%m-%d %H:%M:%S UTC")));
    
    if include_metadata {
        content.push_str(&format!("**Think Mode:** {}\n", if session.metadata.think_mode { "Enabled" } else { "Disabled" }));
        content.push_str(&format!("**Total Tokens:** {}\n", session.metadata.total_tokens));
        content.push_str(&format!("**Request Count:** {}\n", session.metadata.request_count));
    }
    
    content.push_str("\n---\n\n");
    
    // Messages
    for message in &session.messages {
        let role_name = match message.role.as_str() {
            "user" => "👤 User",
            "assistant" => "🤖 Assistant",
            "system" => "⚙️ System",
            _ => &message.role,
        };
        
        content.push_str(&format!("### {}\n\n", role_name));
        
        if let Some(content_str) = message.content.as_str() {
            content.push_str(content_str);
        } else {
            content.push_str(&format!("```json\n{}\n```", serde_json::to_string_pretty(&message.content).unwrap_or_default()));
        }
        
        if include_timestamps {
            content.push_str(&format!("\n\n*{}*", session.created_at.format("%H:%M:%S")));
        }
        
        content.push_str("\n\n---\n\n");
    }
    
    content
}

fn export_as_text(
    session: &crate::server::state::ChatSession,
    include_metadata: bool,
    include_timestamps: bool,
) -> String {
    let mut content = String::new();
    
    // Header
    content.push_str(&format!("Chat Session: {}\n", session.id));
    content.push_str(&format!("Created: {}\n", session.created_at.format("%Y-%m-%d %H:%M:%S UTC")));
    
    if include_metadata {
        content.push_str(&format!("Think Mode: {}\n", if session.metadata.think_mode { "Enabled" } else { "Disabled" }));
        content.push_str(&format!("Total Tokens: {}\n", session.metadata.total_tokens));
        content.push_str(&format!("Request Count: {}\n", session.metadata.request_count));
    }
    
    content.push_str("\n");
    
    // Messages
    for message in &session.messages {
        let role_name = match message.role.as_str() {
            "user" => "User",
            "assistant" => "Assistant",
            "system" => "System",
            _ => &message.role,
        };
        
        content.push_str(&format!("[{}] ", role_name));
        
        if let Some(content_str) = message.content.as_str() {
            content.push_str(content_str);
        } else {
            content.push_str(&format!("[JSON: {}]", serde_json::to_string(&message.content).unwrap_or_default()));
        }
        
        if include_timestamps {
            content.push_str(&format!(" ({})", session.created_at.format("%H:%M:%S")));
        }
        
        content.push_str("\n\n");
    }
    
    content
}

fn export_as_html(
    session: &crate::server::state::ChatSession,
    include_metadata: bool,
    include_timestamps: bool,
) -> String {
    let mut content = String::new();
    
    // HTML header
    content.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
    content.push_str(&format!("<title>Chat Session: {}</title>\n", session.id));
    content.push_str("<style>\n");
    content.push_str("body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }\n");
    content.push_str(".message { margin: 20px 0; padding: 15px; border-radius: 8px; }\n");
    content.push_str(".user { background: #e3f2fd; }\n");
    content.push_str(".assistant { background: #f5f5f5; }\n");
    content.push_str(".system { background: #fff3e0; }\n");
    content.push_str(".timestamp { color: #666; font-size: 0.9em; }\n");
    content.push_str("pre { background: #f5f5f5; padding: 10px; border-radius: 4px; overflow-x: auto; }\n");
    content.push_str("</style>\n");
    content.push_str("</head>\n<body>\n");
    
    // Header
    content.push_str(&format!("<h1>Chat Session: {}</h1>\n", session.id));
    content.push_str(&format!("<p><strong>Created:</strong> {}</p>\n", session.created_at.format("%Y-%m-%d %H:%M:%S UTC")));
    
    if include_metadata {
        content.push_str(&format!("<p><strong>Think Mode:</strong> {}</p>\n", if session.metadata.think_mode { "Enabled" } else { "Disabled" }));
        content.push_str(&format!("<p><strong>Total Tokens:</strong> {}</p>\n", session.metadata.total_tokens));
        content.push_str(&format!("<p><strong>Request Count:</strong> {}</p>\n", session.metadata.request_count));
    }
    
    content.push_str("<hr>\n");
    
    // Messages
    for message in &session.messages {
        let role_class = match message.role.as_str() {
            "user" => "user",
            "assistant" => "assistant",
            "system" => "system",
            _ => "system",
        };
        
        let role_name = match message.role.as_str() {
            "user" => "User",
            "assistant" => "Assistant",
            "system" => "System",
            _ => &message.role,
        };
        
        content.push_str(&format!("<div class=\"message {}\">\n", role_class));
        content.push_str(&format!("<h3>{}</h3>\n", role_name));
        
        if let Some(content_str) = message.content.as_str() {
            // Simple text content
            content.push_str(&format!("<p>{}</p>\n", crate::server::utils::html_escape(content_str)));
        } else {
            // JSON content
            content.push_str(&format!("<pre>{}</pre>\n", crate::server::utils::html_escape(&serde_json::to_string_pretty(&message.content).unwrap_or_default())));
        }
        
        if include_timestamps {
            content.push_str(&format!("<p class=\"timestamp\">{}</p>\n", session.created_at.format("%H:%M:%S")));
        }
        
        content.push_str("</div>\n");
    }
    
    content.push_str("</body>\n</html>");
    content
}
