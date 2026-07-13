//! Concurrent tool-call result cache with TTL and FIFO eviction.

use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, SystemTime},
};

use serde_json::Value;

/// Cache key for tool calls
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct CacheKey {
    /// Name of the tool.
    tool_name: String,
    /// Canonically serialized arguments. Object keys and string values are
    /// preserved exactly.
    arguments: String,
}

impl CacheKey {
    /// Create a cache key from a tool name and its (arbitrary JSON) arguments.
    #[cfg(test)]
    fn new(tool_name: String, arguments: Value) -> Self {
        Self {
            tool_name,
            arguments: canonical_json(&arguments),
        }
    }

    /// Build an executor cache key tied to one registration generation.
    pub(crate) fn for_generation(tool_name: String, arguments: Value, generation: u64) -> Self {
        Self {
            tool_name,
            arguments: format!("{generation}:{}", canonical_json(&arguments)),
        }
    }
}

/// Cache entry with TTL
#[derive(Debug, Clone)]
struct CacheEntry {
    /// Cached tool result.
    result: Value,
    /// When the entry was inserted.
    timestamp: SystemTime,
    /// Time-to-live for this entry.
    ttl: Duration,
}

impl CacheEntry {
    /// Create a new cache entry with the given result and TTL.
    fn new(result: Value, ttl: Duration) -> Self {
        Self {
            result,
            timestamp: SystemTime::now(),
            ttl,
        }
    }

    /// Whether this entry has exceeded its TTL.
    fn is_expired(&self) -> bool {
        match self.timestamp.elapsed() {
            Ok(elapsed) => elapsed >= self.ttl,
            Err(_) => true,
        }
    }
}

/// Concurrent (`DashMap`-backed) cache of tool-call results with per-entry TTL
/// and bounded FIFO eviction. Cloning is cheap
/// (an `Arc` bump) — all clones share the same cached entries, so a
/// [`ToolExecutor`](crate::toolkits::executor::ToolExecutor) cloned per tool
/// call does not deep-copy the cache.
#[derive(Clone)]
pub(crate) struct ToolCallCache {
    /// Shared mutable cache contents (entries + eviction ordering).
    state: Arc<CacheState>,
    default_ttl: Duration,
    max_size: usize,
    enable_cache: bool,
}

/// The shared, concurrent interior of [`ToolCallCache`].
struct CacheState {
    entries: dashmap::DashMap<CacheKey, StoredEntry>,
    /// Reads do not refresh insertion order. Generation tags make stale queue
    /// records harmless after expiry, replacement, or invalidation.
    insertion_order: Mutex<VecDeque<(CacheKey, u64)>>,
    next_generation: AtomicU64,
    total_hits: AtomicU64,
    total_misses: AtomicU64,
}

struct StoredEntry {
    value: CacheEntry,
    generation: u64,
}

impl ToolCallCache {
    /// Create a new cache (default TTL 300s, max 1000 entries, enabled).
    pub(crate) fn new() -> Self {
        Self {
            state: Arc::new(CacheState {
                entries: dashmap::DashMap::new(),
                insertion_order: Mutex::new(VecDeque::new()),
                next_generation: AtomicU64::new(0),
                total_hits: AtomicU64::new(0),
                total_misses: AtomicU64::new(0),
            }),
            default_ttl: Duration::from_secs(300),
            max_size: 1000,
            enable_cache: true,
        }
    }

    /// Set the default TTL for entries without an explicit TTL.
    pub(crate) fn with_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = ttl;
        self
    }

    /// Set the maximum number of cached entries.
    pub(crate) fn with_max_size(mut self, size: usize) -> Self {
        self.max_size = size;
        self
    }

    /// Enable or disable the cache entirely.
    pub(crate) fn with_enabled(mut self, enabled: bool) -> Self {
        self.enable_cache = enabled;
        self
    }

    /// Whether the cache is currently enabled (a `get`/`insert` no-op when
    /// false). Lets callers avoid building a [`CacheKey`] — which deep-clones
    /// and re-serializes the arguments — when the cache is disabled.
    pub(crate) fn enabled(&self) -> bool {
        self.enable_cache
    }

    /// Look up a cached result, returning `None` if disabled, missing, or
    /// expired (expired entries are atomically removed).
    pub(crate) fn get(&self, key: &CacheKey) -> Option<Value> {
        if !self.enable_cache {
            return None;
        }

        // Check-and-remove must be atomic: a separate expiry check followed by
        // `remove` could delete a concurrent replacement.
        let expired = self
            .state
            .entries
            .remove_if(key, |_key, stored| stored.value.is_expired());

        if let Some((_, expired)) = expired {
            self.state
                .insertion_order
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .retain(|(queued_key, generation)| {
                    queued_key != key || *generation != expired.generation
                });
            saturating_increment(&self.state.total_misses);
            return None;
        }

        let Some(stored) = self.state.entries.get(key) else {
            saturating_increment(&self.state.total_misses);
            return None;
        };
        saturating_increment(&self.state.total_hits);
        Some(stored.value.result.clone())
    }

    /// Insert a result, evicting the oldest entries at capacity. No-op if
    /// disabled.
    pub(crate) fn insert(&self, key: CacheKey, result: Value, ttl: Option<Duration>) {
        if !self.enable_cache || self.max_size == 0 {
            return;
        }

        // Serialize insertion-order updates and capacity enforcement. Each
        // queue record carries a generation, so a stale record from an expired,
        // invalidated, or replaced key can never remove a newer value.
        let mut order = self
            .state
            .insertion_order
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // A replacement becomes the newest FIFO entry. Removing prior records
        // also bounds the queue under repeated writes to one key.
        order.retain(|(queued_key, _)| queued_key != &key);
        let generation = self.state.next_generation.fetch_add(1, Ordering::Relaxed);
        let entry = CacheEntry::new(result, ttl.unwrap_or(self.default_ttl));
        self.state.entries.insert(
            key.clone(),
            StoredEntry {
                value: entry,
                generation,
            },
        );
        order.push_back((key, generation));

        while self.state.entries.len() > self.max_size {
            let Some((oldest, oldest_generation)) = order.pop_front() else {
                break;
            };
            self.state.entries.remove_if(&oldest, |_key, stored| {
                stored.generation == oldest_generation
            });
        }
    }

    /// Convenience: build a [`CacheKey`] from name+arguments and insert.
    #[cfg(test)]
    fn insert_with_key(&self, tool_name: String, arguments: Value, result: Value) {
        let key = CacheKey::new(tool_name, arguments);
        self.insert(key, result, None);
    }

    /// Remove all cached entries.
    pub(crate) fn clear(&self) {
        let mut order = self
            .state
            .insertion_order
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.state.entries.clear();
        order.clear();
        self.state.total_hits.store(0, Ordering::Relaxed);
        self.state.total_misses.store(0, Ordering::Relaxed);
    }

    /// Invalidate every entry for the given tool.
    pub(crate) fn invalidate_tool(&self, tool_name: &str) {
        let mut order = self
            .state
            .insertion_order
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.state
            .entries
            .retain(|key, _| key.tool_name != tool_name);
        order.retain(|(key, _)| key.tool_name != tool_name);
    }

    /// Compute aggregate cache statistics (entry count, hits, expiry, hit
    /// rate).
    pub(crate) fn stats(&self) -> CacheStats {
        let mut expired_count = 0u64;

        for entry in self.state.entries.iter() {
            if entry.value.is_expired() {
                expired_count += 1;
            }
        }

        let total_entries = self.state.entries.len();
        let total_hits = self.state.total_hits.load(Ordering::Relaxed);
        let total_misses = self.state.total_misses.load(Ordering::Relaxed);
        let total_lookups = total_hits.saturating_add(total_misses);
        CacheStats {
            total_entries,
            total_hits,
            total_misses,
            expired_count,
            hit_rate: if total_lookups == 0 {
                0.0
            } else {
                total_hits as f64 / total_lookups as f64
            },
        }
    }
}

impl Default for ToolCallCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct CacheStats {
    /// Number of entries currently in the cache.
    pub total_entries: usize,
    /// Total cache hits across all entries.
    pub total_hits: u64,
    /// Total enabled-cache lookups that did not find a live entry.
    pub total_misses: u64,
    /// Number of entries that have expired (not yet evicted).
    pub expired_count: u64,
    /// Fraction of enabled-cache lookups that found a live entry.
    pub hit_rate: f64,
}

fn saturating_increment(counter: &AtomicU64) {
    // `try_update` is the nightly name for this operation, but `fetch_update`
    // remains necessary until the crate's Rust 1.88 MSRV can use the rename.
    #[allow(deprecated)]
    let _ = counter.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
        value.checked_add(1)
    });
}

fn canonical_json(value: &Value) -> String {
    // `serde_json::Value`'s Display implementation emits valid compact JSON.
    // With serde_json's default map backend, object keys are sorted, so
    // equivalent objects with different insertion order share a key.
    value.to_string()
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
    fn cache_key_distinguishes_name_input_and_registration_generation() {
        let first = CacheKey::for_generation("one".to_string(), serde_json::json!({"n": 1}), 7);
        let other_name =
            CacheKey::for_generation("two".to_string(), serde_json::json!({"n": 1}), 7);
        let other_input =
            CacheKey::for_generation("one".to_string(), serde_json::json!({"n": 2}), 7);
        let other_generation =
            CacheKey::for_generation("one".to_string(), serde_json::json!({"n": 1}), 8);

        assert_ne!(first, other_name);
        assert_ne!(first, other_input);
        assert_ne!(first, other_generation);
    }

    #[test]
    fn test_cache_entry_expired() {
        let entry = CacheEntry::new(
            serde_json::json!({"result": "success"}),
            Duration::from_secs(1),
        );
        assert!(!entry.is_expired());

        let mut entry_mut = entry;
        entry_mut.timestamp = SystemTime::now() - Duration::from_secs(2);
        assert!(entry_mut.is_expired());
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
        let cache = ToolCallCache::new().with_ttl(Duration::from_millis(10));
        let args = serde_json::json!({"input": "test"});
        let result = serde_json::json!({"output": "success"});

        cache.insert_with_key("test_tool".to_string(), args.clone(), result);

        let key = CacheKey::new("test_tool".to_string(), args);

        assert!(cache.get(&key).is_some());

        std::thread::sleep(Duration::from_millis(20));

        assert!(cache.get(&key).is_none());
    }

    #[test]
    fn test_cache_stats() {
        let cache = ToolCallCache::new();
        let args = serde_json::json!({"input": "test"});

        cache.insert_with_key("tool_a".to_string(), args.clone(), serde_json::json!({}));
        cache.insert_with_key("tool_b".to_string(), args.clone(), serde_json::json!({}));

        let key = CacheKey::new("tool_a".to_string(), args);
        let _ = cache.get(&key);
        let _ = cache.get(&key);
        let _ = cache.get(&CacheKey::new("missing".to_string(), serde_json::json!({})));

        let stats = cache.stats();
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.total_hits, 2);
        assert_eq!(stats.total_misses, 1);
        assert!((stats.hit_rate - (2.0 / 3.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn test_canonical_json() {
        let obj = serde_json::json!({
            "CITY": "Shenzhen",
            "count": 5,
            "Data": {"NAME": "test"}
        });

        let normalized = canonical_json(&obj);
        let parsed: Value = serde_json::from_str(&normalized).unwrap();

        // Keys preserve their exact spelling.
        if let Some(parsed_obj) = parsed.as_object() {
            assert!(parsed_obj.contains_key("CITY"));
            assert!(parsed_obj.contains_key("count"));
            assert!(parsed_obj.contains_key("Data"));
            assert_eq!(parsed_obj.get("CITY"), Some(&serde_json::json!("Shenzhen")));
            assert_eq!(parsed_obj.get("count"), Some(&serde_json::json!(5)));
        }
    }

    #[test]
    fn test_canonical_json_preserves_key_whitespace() {
        let obj = serde_json::json!({"CityName": "Shenzhen", " UserID ": 42});
        let normalized = canonical_json(&obj);
        let parsed: Value = serde_json::from_str(&normalized).unwrap();
        assert!(parsed.as_object().unwrap().contains_key("CityName"));
        assert!(parsed.as_object().unwrap().contains_key(" UserID "));
    }

    #[test]
    fn test_cache_concurrent_insert_and_get() {
        use std::{sync::Arc, thread};

        let cache = Arc::new(ToolCallCache::new().with_max_size(1000));
        let mut handles = Vec::new();

        for i in 0..10 {
            let cache_clone = Arc::clone(&cache);
            handles.push(thread::spawn(move || {
                let key_name = format!("tool_{i}");
                let args = serde_json::json!({"input": i});
                let result = serde_json::json!({"output": format!("result_{}", i)});
                cache_clone.insert_with_key(key_name.clone(), args.clone(), result);

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
    fn test_cache_evicts_at_capacity() {
        let cache = ToolCallCache::new()
            .with_max_size(5)
            .with_ttl(Duration::from_secs(300));

        for i in 0..5 {
            let args = serde_json::json!({"input": i});
            cache.insert_with_key(format!("tool_{i}"), args, serde_json::json!({"result": i}));
        }

        assert_eq!(cache.stats().total_entries, 5);

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

    #[test]
    fn test_cache_max_size_zero_stores_nothing() {
        let cache = ToolCallCache::new().with_max_size(0);
        let args = serde_json::json!({"input": "test"});
        let key = CacheKey::new("tool".to_string(), args.clone());

        cache.insert_with_key(
            "tool".to_string(),
            args,
            serde_json::json!({"result": true}),
        );

        assert!(cache.get(&key).is_none());
        assert_eq!(cache.stats().total_entries, 0);
    }

    #[test]
    fn test_cache_collides_on_reordered_keys() {
        // End-to-end pin: equivalent objects with reordered keys share a key.
        let cache = ToolCallCache::new();
        cache.insert_with_key(
            "t".to_string(),
            serde_json::json!({"a": 1, "b": 2}),
            serde_json::json!(true),
        );
        let reordered = CacheKey::new("t".to_string(), serde_json::json!({"b": 2, "a": 1}));
        assert!(
            cache.get(&reordered).is_some(),
            "reordered object keys must collide in the cache"
        );
    }

    #[test]
    fn test_cache_key_preserves_whitespace_bearing_keys() {
        let messy = CacheKey::new("t".to_string(), serde_json::json!({" a ": 1}));
        let clean = CacheKey::new("t".to_string(), serde_json::json!({"a": 1}));
        assert_ne!(messy, clean);
    }

    #[test]
    fn string_and_json_null_do_not_collide() {
        let string = CacheKey::new("t".to_string(), Value::String("null".to_string()));
        let null = CacheKey::new("t".to_string(), Value::Null);
        assert_ne!(string, null);
    }

    #[test]
    fn invalidation_removes_fifo_record_before_reinsert() {
        let cache = ToolCallCache::new().with_max_size(2);
        let key = CacheKey::new("same".to_string(), serde_json::json!({}));
        cache.insert(key.clone(), serde_json::json!(1), None);
        cache.insert(
            CacheKey::new("older".to_string(), serde_json::json!({})),
            serde_json::json!(0),
            None,
        );
        cache.invalidate_tool("same");
        cache.insert(key.clone(), serde_json::json!(2), None);
        cache.insert(
            CacheKey::new("newest".to_string(), serde_json::json!({})),
            serde_json::json!(3),
            None,
        );
        assert_eq!(cache.get(&key), Some(serde_json::json!(2)));
        assert_eq!(
            cache
                .state
                .insertion_order
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .len(),
            2
        );
    }
}
