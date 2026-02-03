//! Tool call result cache with intelligent invalidation

use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Cache key for tool calls
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CacheKey {
    pub tool_name: String,
    pub arguments: String,
}

impl CacheKey {
    pub fn new(tool_name: String, arguments: Value) -> Self {
        let normalized = normalize_json(&arguments);
        Self {
            tool_name,
            arguments: normalized,
        }
    }
}

/// Cache entry with TTL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub result: Value,
    pub timestamp: SystemTime,
    pub ttl: Duration,
    pub hit_count: u64,
}

impl CacheEntry {
    pub fn new(result: Value, ttl: Duration) -> Self {
        Self {
            result,
            timestamp: SystemTime::now(),
            ttl,
            hit_count: 0,
        }
    }

    pub fn is_expired(&self) -> bool {
        match self.timestamp.elapsed() {
            Ok(elapsed) => elapsed > self.ttl,
            Err(_) => true,
        }
    }

    pub fn hit(&mut self) {
        self.hit_count += 1;
    }
}

/// Intelligent tool call result cache
#[derive(Clone)]
pub struct ToolCallCache {
    entries: dashmap::DashMap<CacheKey, CacheEntry>,
    default_ttl: Duration,
    max_size: usize,
    enable_cache: bool,
}

impl ToolCallCache {
    pub fn new() -> Self {
        Self {
            entries: dashmap::DashMap::new(),
            default_ttl: Duration::from_secs(300),
            max_size: 1000,
            enable_cache: true,
        }
    }

    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = ttl;
        self
    }

    pub fn with_max_size(mut self, size: usize) -> Self {
        self.max_size = size;
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enable_cache = enabled;
        self
    }

    pub fn get(&self, key: &CacheKey) -> Option<Value> {
        if !self.enable_cache {
            return None;
        }

        // First check if entry exists and is expired without holding the lock
        let expired = {
            let entry = self.entries.get(key)?;
            entry.is_expired()
        };

        // If expired, remove it and return None
        if expired {
            self.entries.remove(key);
            return None;
        }

        // Get mut reference for hit counting and result cloning
        let mut entry = self.entries.get_mut(key)?;
        entry.hit();
        Some(entry.result.clone())
    }

    pub fn insert(&self, key: CacheKey, result: Value, ttl: Option<Duration>) {
        if !self.enable_cache {
            return;
        }

        if self.entries.len() >= self.max_size {
            self.evict_lru();
        }

        let entry = CacheEntry::new(result, ttl.unwrap_or(self.default_ttl));
        self.entries.insert(key, entry);
    }

    pub fn insert_with_key(&self, tool_name: String, arguments: Value, result: Value) {
        let key = CacheKey::new(tool_name, arguments);
        self.insert(key, result, None);
    }

    pub fn clear(&self) {
        self.entries.clear();
    }

    pub fn invalidate_tool(&self, tool_name: &str) {
        self.entries.retain(|key, _| key.tool_name != tool_name);
    }

    pub fn stats(&self) -> CacheStats {
        let mut total_hits = 0u64;
        let mut expired_count = 0u64;

        for entry in self.entries.iter() {
            total_hits += entry.hit_count;
            if entry.is_expired() {
                expired_count += 1;
            }
        }

        CacheStats {
            total_entries: self.entries.len(),
            total_hits,
            expired_count,
            hit_rate: if self.entries.is_empty() {
                0.0
            } else {
                total_hits as f64 / self.entries.len() as f64
            },
        }
    }

    fn evict_lru(&self) {
        let mut entries: Vec<_> = self
            .entries
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().timestamp))
            .collect();

        entries.sort_by_key(|a| a.1);

        let remove_count = (self.max_size / 10).max(1);
        for (key, _) in entries.into_iter().take(remove_count) {
            self.entries.remove(&key);
        }
    }
}

impl Default for ToolCallCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_hits: u64,
    pub expired_count: u64,
    pub hit_rate: f64,
}

fn normalize_json(value: &Value) -> String {
    match value {
        Value::Object(obj) => {
            let mut normalized = serde_json::Map::new();
            for (k, v) in obj {
                let normalized_key = k.trim().to_lowercase();
                let normalized_value = normalize_json_value(v);
                normalized.insert(normalized_key, normalized_value);
            }
            serde_json::to_string(&normalized).unwrap_or_default()
        },
        Value::Array(arr) => {
            let normalized: Vec<_> = arr.iter().map(normalize_json_value).collect();
            serde_json::to_string(&normalized).unwrap_or_default()
        },
        Value::String(s) => s.clone(),
        _ => serde_json::to_string(value).unwrap_or_default(),
    }
}

fn normalize_json_value(value: &Value) -> Value {
    match value {
        Value::Object(obj) => {
            let mut normalized = serde_json::Map::new();
            for (k, v) in obj {
                let normalized_key = k.trim().to_lowercase();
                normalized.insert(normalized_key, normalize_json_value(v));
            }
            Value::Object(normalized)
        },
        Value::Array(arr) => {
            let normalized: Vec<_> = arr.iter().map(normalize_json_value).collect();
            Value::Array(normalized)
        },
        _ => value.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_new() {
        let args = serde_json::json!({"city": "Shenzhen", "count": 5});
        let key = CacheKey::new("test_tool".to_string(), args);
        assert_eq!(key.tool_name, "test_tool");
        assert!(key.arguments.contains("city"));
    }

    #[test]
    fn test_cache_entry_expired() {
        let entry = CacheEntry::new(
            serde_json::json!({"result": "success"}),
            Duration::from_secs(1),
        );
        assert!(!entry.is_expired());

        let mut entry_mut = entry.clone();
        entry_mut.timestamp = SystemTime::now() - Duration::from_secs(2);
        assert!(entry_mut.is_expired());
    }

    #[test]
    fn test_cache_hit() {
        let mut entry = CacheEntry::new(
            serde_json::json!({"result": "success"}),
            Duration::from_secs(60),
        );
        entry.hit();
        entry.hit();
        assert_eq!(entry.hit_count, 2);
    }

    #[test]
    fn test_cache_insert_get() {
        let cache = ToolCallCache::new();
        let args = serde_json::json!({"input": "test"});
        let result = serde_json::json!({"output": "success"});

        cache.insert_with_key("test_tool".to_string(), args.clone(), result.clone());

        let key = CacheKey::new("test_tool".to_string(), args);
        let cached = cache.get(&key);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), result);
    }

    #[test]
    fn test_cache_expiration() {
        // Test expiration with short TTL and sleep
        let cache = ToolCallCache::new().with_ttl(Duration::from_millis(10));
        let args = serde_json::json!({"input": "test"});
        let result = serde_json::json!({"output": "success"});

        cache.insert_with_key("test_tool".to_string(), args.clone(), result.clone());

        let key = CacheKey::new("test_tool".to_string(), args.clone());

        // Entry should be cached initially
        assert!(cache.get(&key).is_some());

        // Wait for TTL to expire
        std::thread::sleep(Duration::from_millis(20));

        // Entry should be expired now
        assert!(cache.get(&key).is_none());
    }

    #[test]
    fn test_cache_stats() {
        let cache = ToolCallCache::new();
        let args = serde_json::json!({"input": "test"});

        cache.insert_with_key("tool_a".to_string(), args.clone(), serde_json::json!({}));
        cache.insert_with_key("tool_b".to_string(), args.clone(), serde_json::json!({}));

        let key = CacheKey::new("tool_a".to_string(), args.clone());
        let _ = cache.get(&key);
        let _ = cache.get(&key);

        let stats = cache.stats();
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.total_hits, 2);
    }

    #[test]
    fn test_normalize_json() {
        let obj = serde_json::json!({
            "CITY": "Shenzhen",
            "count": 5,
            "Data": {"NAME": "test"}
        });

        let normalized = normalize_json(&obj);
        let parsed: Value = serde_json::from_str(&normalized).unwrap();

        // Keys should be lowercase, but values should be preserved
        if let Some(parsed_obj) = parsed.as_object() {
            assert!(parsed_obj.contains_key("city"));
            assert!(parsed_obj.contains_key("count"));
            assert!(parsed_obj.contains_key("data"));
            assert_eq!(parsed_obj.get("city"), Some(&serde_json::json!("Shenzhen")));
            assert_eq!(parsed_obj.get("count"), Some(&serde_json::json!(5)));
        }
    }
}
