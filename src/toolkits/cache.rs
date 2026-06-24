//! Tool call result cache with intelligent invalidation

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Cache key for tool calls
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CacheKey {
    /// Name of the tool.
    pub tool_name: String,
    /// Normalized (whitespace-trimmed) JSON arguments string.
    pub arguments: String,
}

impl CacheKey {
    /// Create a cache key from a tool name and its (arbitrary JSON) arguments.
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
    /// Cached tool result.
    pub result: Value,
    /// When the entry was inserted.
    pub timestamp: SystemTime,
    /// Time-to-live for this entry.
    pub ttl: Duration,
    /// Number of cache hits on this entry.
    pub hit_count: u64,
}

impl CacheEntry {
    /// Create a new cache entry with the given result and TTL.
    pub fn new(result: Value, ttl: Duration) -> Self {
        Self {
            result,
            timestamp: SystemTime::now(),
            ttl,
            hit_count: 0,
        }
    }

    /// Whether this entry has exceeded its TTL.
    pub fn is_expired(&self) -> bool {
        match self.timestamp.elapsed() {
            Ok(elapsed) => elapsed > self.ttl,
            Err(_) => true,
        }
    }

    /// Record one cache hit on this entry.
    pub fn hit(&mut self) {
        self.hit_count += 1;
    }
}

/// Intelligent tool call result cache
///
/// Concurrent (`DashMap`-backed) cache of tool-call results with per-entry TTL,
/// O(1) FIFO eviction at capacity, and hit/miss statistics. Cloning is cheap
/// (an `Arc` bump) — all clones share the same cached entries, so a
/// [`ToolExecutor`](crate::toolkits::executor::ToolExecutor) cloned per tool
/// call does not deep-copy the cache.
#[derive(Clone)]
pub struct ToolCallCache {
    /// Shared mutable cache contents (entries + eviction ordering).
    state: Arc<CacheState>,
    default_ttl: Duration,
    max_size: usize,
    enable_cache: bool,
}

/// The shared, concurrent interior of [`ToolCallCache`].
struct CacheState {
    entries: dashmap::DashMap<CacheKey, CacheEntry>,
    /// Insertion-order queue driving O(1) FIFO eviction (see
    /// [`ToolCallCache::evict_oldest`]). Mirrors the prior timestamp-based
    /// eviction, which was insertion-ordered since `get` does not refresh the
    /// timestamp. Stale keys (removed via expiry/invalidate before eviction)
    /// are skipped lazily.
    insertion_order: Mutex<VecDeque<CacheKey>>,
}

impl ToolCallCache {
    /// Create a new cache (default TTL 300s, max 1000 entries, enabled).
    pub fn new() -> Self {
        Self {
            state: Arc::new(CacheState {
                entries: dashmap::DashMap::new(),
                insertion_order: Mutex::new(VecDeque::new()),
            }),
            default_ttl: Duration::from_secs(300),
            max_size: 1000,
            enable_cache: true,
        }
    }

    /// Set the default TTL for entries without an explicit TTL.
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = ttl;
        self
    }

    /// Set the maximum number of cached entries.
    pub fn with_max_size(mut self, size: usize) -> Self {
        self.max_size = size;
        self
    }

    /// Enable or disable the cache entirely.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enable_cache = enabled;
        self
    }

    /// Look up a cached result, returning `None` if disabled, missing, or
    /// expired (expired entries are atomically removed).
    pub fn get(&self, key: &CacheKey) -> Option<Value> {
        if !self.enable_cache {
            return None;
        }

        // Use DashMap's remove_if for atomic check-and-remove of expired entries.
        // If the entry exists and is expired, atomically remove it and return None.
        // If not expired, we need to get it again for hit counting.
        // This avoids TOCTOU issues between check and remove.
        let expired = self.state.entries.remove_if(key, |_k, v| v.is_expired());

        if expired.is_some() {
            // Entry was expired and removed atomically
            return None;
        }

        // Entry was not expired (or didn't exist) - get it for hit counting
        let mut entry = self.state.entries.get_mut(key)?;
        entry.hit();
        Some(entry.result.clone())
    }

    /// Insert a result, evicting the oldest entries at capacity. No-op if
    /// disabled.
    pub fn insert(&self, key: CacheKey, result: Value, ttl: Option<Duration>) {
        if !self.enable_cache {
            return;
        }

        if self.state.entries.len() >= self.max_size {
            self.evict_oldest();
        }

        // Track insertion order only for genuinely new keys so the eviction
        // queue never carries duplicates. `contains_key` releases its read lock
        // before the write lock is taken below, and the queue mutex is taken
        // only after the entry write lock is released — so there is no
        // lock-ordering cycle with `evict_oldest` (which holds the queue mutex
        // then takes entry locks).
        let was_present = self.state.entries.contains_key(&key);
        let entry = CacheEntry::new(result, ttl.unwrap_or(self.default_ttl));
        self.state.entries.insert(key.clone(), entry);
        if !was_present {
            if let Ok(mut order) = self.state.insertion_order.lock() {
                order.push_back(key);
            }
        }
    }

    /// Convenience: build a [`CacheKey`] from name+arguments and insert.
    pub fn insert_with_key(&self, tool_name: String, arguments: Value, result: Value) {
        let key = CacheKey::new(tool_name, arguments);
        self.insert(key, result, None);
    }

    /// Remove all cached entries.
    pub fn clear(&self) {
        self.state.entries.clear();
        if let Ok(mut order) = self.state.insertion_order.lock() {
            order.clear();
        }
    }

    /// Invalidate every entry for the given tool.
    pub fn invalidate_tool(&self, tool_name: &str) {
        self.state
            .entries
            .retain(|key, _| key.tool_name != tool_name);
    }

    /// Compute aggregate cache statistics (entry count, hits, expiry, hit
    /// rate).
    pub fn stats(&self) -> CacheStats {
        let mut total_hits = 0u64;
        let mut expired_count = 0u64;

        for entry in self.state.entries.iter() {
            total_hits += entry.hit_count;
            if entry.is_expired() {
                expired_count += 1;
            }
        }

        let total_entries = self.state.entries.len();
        CacheStats {
            total_entries,
            total_hits,
            expired_count,
            hit_rate: if total_entries == 0 {
                0.0
            } else {
                total_hits as f64 / total_entries as f64
            },
        }
    }

    /// Evict the oldest ~10% of entries in O(1) amortized time.
    ///
    /// Pops from the front of the insertion-order queue (oldest first) and
    /// removes the corresponding entries. Keys already removed (expired during
    /// a `get`, or invalidated) are skipped without counting toward the
    /// eviction budget, so the queue self-cleans.
    fn evict_oldest(&self) {
        let mut budget = (self.max_size / 10).max(1);
        let mut order = match self.state.insertion_order.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        while budget > 0 {
            let Some(key) = order.pop_front() else { break };
            // Only consume budget for entries still present — stale queue
            // entries (removed via expiry/invalidate) drop silently.
            if self.state.entries.remove(&key).is_some() {
                budget -= 1;
            }
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
    /// Number of entries currently in the cache.
    pub total_entries: usize,
    /// Total cache hits across all entries.
    pub total_hits: u64,
    /// Number of entries that have expired (not yet evicted).
    pub expired_count: u64,
    /// Hit rate (`total_hits / total_entries`).
    pub hit_rate: f64,
}

fn normalize_json(value: &Value) -> String {
    match value {
        Value::Object(obj) => {
            let mut normalized = serde_json::Map::new();
            for (k, v) in obj {
                let normalized_key = k.trim().to_string();
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
                let normalized_key = k.trim().to_string();
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

        // Keys should preserve original case (only trim whitespace)
        if let Some(parsed_obj) = parsed.as_object() {
            assert!(parsed_obj.contains_key("CITY"));
            assert!(parsed_obj.contains_key("count"));
            assert!(parsed_obj.contains_key("Data"));
            assert_eq!(parsed_obj.get("CITY"), Some(&serde_json::json!("Shenzhen")));
            assert_eq!(parsed_obj.get("count"), Some(&serde_json::json!(5)));
        }
    }

    #[test]
    fn test_normalize_json_consistency_with_llm() {
        // Verify that normalize_json preserves case consistently with
        // llm::normalize_arguments (both only trim, no case change)
        let obj = serde_json::json!({"CityName": "Shenzhen", " UserID ": 42});
        let normalized = normalize_json(&obj);
        let parsed: Value = serde_json::from_str(&normalized).unwrap();
        assert!(parsed.as_object().unwrap().contains_key("CityName"));
        assert!(parsed.as_object().unwrap().contains_key("UserID"));
    }

    #[test]
    fn test_cache_concurrent_insert_and_get() {
        use std::{sync::Arc, thread};

        let cache = Arc::new(ToolCallCache::new().with_max_size(1000));
        let mut handles = Vec::new();

        for i in 0..10 {
            let cache_clone = Arc::clone(&cache);
            handles.push(thread::spawn(move || {
                let key_name = format!("tool_{}", i);
                let args = serde_json::json!({"input": i});
                let result = serde_json::json!({"output": format!("result_{}", i)});
                cache_clone.insert_with_key(key_name.clone(), args.clone(), result);

                // Read it back
                let key = CacheKey::new(key_name, args);
                cache_clone.get(&key)
            }));
        }

        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        let successful_gets = results.iter().filter(|r| r.is_some()).count();
        assert_eq!(successful_gets, 10);

        let stats = cache.stats();
        assert_eq!(stats.total_entries, 10);
    }

    #[test]
    fn test_cache_evict_lru() {
        // Create a cache with small max_size
        let cache = ToolCallCache::new()
            .with_max_size(5)
            .with_ttl(Duration::from_secs(300));

        // Insert 5 entries to fill the cache
        for i in 0..5 {
            let args = serde_json::json!({"input": i});
            cache.insert_with_key(
                format!("tool_{}", i),
                args,
                serde_json::json!({"result": i}),
            );
        }

        assert_eq!(cache.stats().total_entries, 5);

        // Insert one more to trigger eviction
        let args = serde_json::json!({"input": "new"});
        cache.insert_with_key(
            "tool_new".to_string(),
            args,
            serde_json::json!({"result": "new"}),
        );

        let stats = cache.stats();
        // After eviction, some entries should have been removed
        assert!(stats.total_entries <= 5);
        // The new entry should be present
        let key = CacheKey::new("tool_new".to_string(), serde_json::json!({"input": "new"}));
        assert!(cache.get(&key).is_some());
    }

    #[test]
    fn test_cache_clone_shares_state() {
        // Cloning a cache is a cheap `Arc` bump and shares the entries, so a
        // write through one handle is visible through the other (a
        // `ToolExecutor` cloned per tool call does not deep-copy the cache).
        let cache = ToolCallCache::new();
        let other = cache.clone();

        let args = serde_json::json!({"input": "shared"});
        cache.insert_with_key(
            "tool".to_string(),
            args.clone(),
            serde_json::json!({"v": 1}),
        );

        let key = CacheKey::new("tool".to_string(), args);
        assert!(cache.get(&key).is_some());
        // Visible through the clone.
        assert!(other.get(&key).is_some());
    }

    #[test]
    fn test_cache_evicts_in_fifo_order() {
        // O(1) FIFO eviction: with max_size 3, inserting a 4th entry evicts
        // the oldest (t0), leaving t1/t2/t3.
        let cache = ToolCallCache::new().with_max_size(3);
        for i in 0..3 {
            cache.insert_with_key(
                format!("t{i}"),
                serde_json::json!({"i": i}),
                serde_json::json!({}),
            );
        }
        assert_eq!(cache.stats().total_entries, 3);

        cache.insert_with_key(
            "t3".to_string(),
            serde_json::json!({"i": 3}),
            serde_json::json!({}),
        );

        assert!(
            cache
                .get(&CacheKey::new(
                    "t0".to_string(),
                    serde_json::json!({"i": 0})
                ))
                .is_none(),
            "oldest entry (t0) should have been evicted"
        );
        assert!(
            cache
                .get(&CacheKey::new(
                    "t1".to_string(),
                    serde_json::json!({"i": 1})
                ))
                .is_some()
        );
        assert!(
            cache
                .get(&CacheKey::new(
                    "t3".to_string(),
                    serde_json::json!({"i": 3})
                ))
                .is_some()
        );
        assert!(cache.stats().total_entries <= 3);
    }
}
