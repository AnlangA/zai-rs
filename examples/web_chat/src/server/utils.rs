//! Server utilities

/// Escape HTML characters in text
pub fn html_escape(text: &str) -> String {
    let mut result = String::with_capacity(text.len() * 2);
    for ch in text.chars() {
        match ch {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&#x27;"),
            _ => result.push(ch),
        }
    }
    result
}

/// Generate a unique session ID
pub fn generate_session_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Format duration in human-readable format
pub fn format_duration(seconds: i64) -> String {
    if seconds < 60 {
        let mut result = seconds.to_string();
        result.push_str(" seconds");
        result
    } else if seconds < 3600 {
        let minutes = seconds / 60;
        let secs = seconds % 60;
        let mut result = minutes.to_string();
        result.push_str(" minutes ");
        result.push_str(&secs.to_string());
        result.push_str(" seconds");
        result
    } else {
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        let mut result = hours.to_string();
        result.push_str(" hours ");
        result.push_str(&minutes.to_string());
        result.push_str(" minutes");
        result
    }
}

/// Truncate text to specified length
pub fn truncate_text(text: &str, max_length: usize) -> String {
    if text.len() <= max_length {
        text.to_string()
    } else {
        let truncate_len = max_length.saturating_sub(3);
        let prefix = &text[..truncate_len];
        let mut result = String::from(prefix);
        result.push_str("...");
        result
    }
}
