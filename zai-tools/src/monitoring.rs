//! Monitoring and metrics collection for ZAI Tools
//! 
//! This module provides comprehensive monitoring capabilities including
//! execution metrics, performance tracking, and health monitoring.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

#[cfg(feature = "tracing")]
use tracing::{info, warn, error, debug, span, Level};

/// Metrics collector for tool execution
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    inner: Arc<Mutex<MetricsInner>>,
}

#[derive(Debug)]
struct MetricsInner {
    tool_metrics: HashMap<String, ToolMetrics>,
    global_metrics: GlobalMetrics,
}

/// Metrics for a specific tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetrics {
    /// Tool name
    pub name: String,
    /// Total number of executions
    pub total_executions: u64,
    /// Number of successful executions
    pub successful_executions: u64,
    /// Number of failed executions
    pub failed_executions: u64,
    /// Total execution time
    pub total_execution_time: Duration,
    /// Average execution time
    pub average_execution_time: Duration,
    /// Minimum execution time
    pub min_execution_time: Duration,
    /// Maximum execution time
    pub max_execution_time: Duration,
    /// Last execution time
    pub last_execution: Option<Instant>,
    /// Error counts by error type
    pub error_counts: HashMap<String, u64>,
}

/// Global metrics across all tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalMetrics {
    /// Total number of tool executions
    pub total_executions: u64,
    /// Total successful executions
    pub total_successful: u64,
    /// Total failed executions
    pub total_failed: u64,
    /// Total execution time across all tools
    pub total_execution_time: Duration,
    /// Number of parallel executions
    pub parallel_executions: u64,
    /// Registry operations count
    pub registry_operations: u64,
    /// Start time of metrics collection
    pub start_time: Instant,
}

/// Execution event for metrics collection
#[derive(Debug, Clone)]
pub struct ExecutionEvent {
    /// Tool name
    pub tool_name: String,
    /// Execution duration
    pub duration: Duration,
    /// Whether execution was successful
    pub success: bool,
    /// Error type if execution failed
    pub error_type: Option<String>,
    /// Timestamp of execution
    pub timestamp: Instant,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(MetricsInner {
                tool_metrics: HashMap::new(),
                global_metrics: GlobalMetrics {
                    total_executions: 0,
                    total_successful: 0,
                    total_failed: 0,
                    total_execution_time: Duration::from_secs(0),
                    parallel_executions: 0,
                    registry_operations: 0,
                    start_time: Instant::now(),
                },
            })),
        }
    }
    
    /// Record an execution event
    pub fn record_execution(&self, event: ExecutionEvent) {
        let mut inner = self.inner.lock().unwrap();
        
        // Update global metrics
        inner.global_metrics.total_executions += 1;
        if event.success {
            inner.global_metrics.total_successful += 1;
        } else {
            inner.global_metrics.total_failed += 1;
        }
        inner.global_metrics.total_execution_time += event.duration;
        
        // Update tool-specific metrics
        let tool_metrics = inner.tool_metrics
            .entry(event.tool_name.clone())
            .or_insert_with(|| ToolMetrics::new(event.tool_name.clone()));
        
        tool_metrics.record_execution(event);
        
        #[cfg(feature = "tracing")]
        {
            let span = span!(Level::DEBUG, "tool_execution", tool = %event.tool_name);
            let _enter = span.enter();
            
            if event.success {
                debug!(
                    duration_ms = event.duration.as_millis(),
                    "Tool execution completed successfully"
                );
            } else {
                warn!(
                    duration_ms = event.duration.as_millis(),
                    error_type = event.error_type.as_deref().unwrap_or("unknown"),
                    "Tool execution failed"
                );
            }
        }
    }
    
    /// Record a parallel execution
    pub fn record_parallel_execution(&self, count: usize) {
        let mut inner = self.inner.lock().unwrap();
        inner.global_metrics.parallel_executions += count as u64;
        
        #[cfg(feature = "tracing")]
        info!(parallel_count = count, "Parallel execution completed");
    }
    
    /// Record a registry operation
    pub fn record_registry_operation(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.global_metrics.registry_operations += 1;
    }
    
    /// Get metrics for a specific tool
    pub fn get_tool_metrics(&self, tool_name: &str) -> Option<ToolMetrics> {
        let inner = self.inner.lock().unwrap();
        inner.tool_metrics.get(tool_name).cloned()
    }
    
    /// Get global metrics
    pub fn get_global_metrics(&self) -> GlobalMetrics {
        let inner = self.inner.lock().unwrap();
        inner.global_metrics.clone()
    }
    
    /// Get all tool metrics
    pub fn get_all_tool_metrics(&self) -> HashMap<String, ToolMetrics> {
        let inner = self.inner.lock().unwrap();
        inner.tool_metrics.clone()
    }
    
    /// Generate a metrics report
    pub fn generate_report(&self) -> MetricsReport {
        let inner = self.inner.lock().unwrap();
        let uptime = inner.global_metrics.start_time.elapsed();
        
        MetricsReport {
            global_metrics: inner.global_metrics.clone(),
            tool_metrics: inner.tool_metrics.clone(),
            uptime,
            generated_at: Instant::now(),
        }
    }
    
    /// Reset all metrics
    pub fn reset(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.tool_metrics.clear();
        inner.global_metrics = GlobalMetrics {
            total_executions: 0,
            total_successful: 0,
            total_failed: 0,
            total_execution_time: Duration::from_secs(0),
            parallel_executions: 0,
            registry_operations: 0,
            start_time: Instant::now(),
        };
        
        #[cfg(feature = "tracing")]
        info!("Metrics reset");
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolMetrics {
    fn new(name: String) -> Self {
        Self {
            name,
            total_executions: 0,
            successful_executions: 0,
            failed_executions: 0,
            total_execution_time: Duration::from_secs(0),
            average_execution_time: Duration::from_secs(0),
            min_execution_time: Duration::from_secs(u64::MAX),
            max_execution_time: Duration::from_secs(0),
            last_execution: None,
            error_counts: HashMap::new(),
        }
    }
    
    fn record_execution(&mut self, event: ExecutionEvent) {
        self.total_executions += 1;
        self.total_execution_time += event.duration;
        self.last_execution = Some(event.timestamp);
        
        if event.success {
            self.successful_executions += 1;
        } else {
            self.failed_executions += 1;
            if let Some(error_type) = event.error_type {
                *self.error_counts.entry(error_type).or_insert(0) += 1;
            }
        }
        
        // Update min/max execution times
        if event.duration < self.min_execution_time {
            self.min_execution_time = event.duration;
        }
        if event.duration > self.max_execution_time {
            self.max_execution_time = event.duration;
        }
        
        // Update average execution time
        self.average_execution_time = self.total_execution_time / self.total_executions as u32;
    }
    
    /// Get success rate as a percentage
    pub fn success_rate(&self) -> f64 {
        if self.total_executions == 0 {
            0.0
        } else {
            (self.successful_executions as f64 / self.total_executions as f64) * 100.0
        }
    }
    
    /// Get failure rate as a percentage
    pub fn failure_rate(&self) -> f64 {
        100.0 - self.success_rate()
    }
    
    /// Get executions per second (based on uptime)
    pub fn executions_per_second(&self, uptime: Duration) -> f64 {
        if uptime.as_secs() == 0 {
            0.0
        } else {
            self.total_executions as f64 / uptime.as_secs_f64()
        }
    }
}

/// Comprehensive metrics report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsReport {
    /// Global metrics
    pub global_metrics: GlobalMetrics,
    /// Per-tool metrics
    pub tool_metrics: HashMap<String, ToolMetrics>,
    /// System uptime
    pub uptime: Duration,
    /// When the report was generated
    pub generated_at: Instant,
}

impl MetricsReport {
    /// Get the most frequently used tool
    pub fn most_used_tool(&self) -> Option<&ToolMetrics> {
        self.tool_metrics
            .values()
            .max_by_key(|metrics| metrics.total_executions)
    }
    
    /// Get the fastest tool (by average execution time)
    pub fn fastest_tool(&self) -> Option<&ToolMetrics> {
        self.tool_metrics
            .values()
            .filter(|metrics| metrics.total_executions > 0)
            .min_by_key(|metrics| metrics.average_execution_time)
    }
    
    /// Get the slowest tool (by average execution time)
    pub fn slowest_tool(&self) -> Option<&ToolMetrics> {
        self.tool_metrics
            .values()
            .filter(|metrics| metrics.total_executions > 0)
            .max_by_key(|metrics| metrics.average_execution_time)
    }
    
    /// Get overall success rate
    pub fn overall_success_rate(&self) -> f64 {
        if self.global_metrics.total_executions == 0 {
            0.0
        } else {
            (self.global_metrics.total_successful as f64 / self.global_metrics.total_executions as f64) * 100.0
        }
    }
    
    /// Get overall executions per second
    pub fn overall_executions_per_second(&self) -> f64 {
        if self.uptime.as_secs() == 0 {
            0.0
        } else {
            self.global_metrics.total_executions as f64 / self.uptime.as_secs_f64()
        }
    }
}

/// Health check status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Overall health status
    pub status: HealthStatus,
    /// Health check timestamp
    pub timestamp: Instant,
    /// Individual check results
    pub checks: HashMap<String, CheckResult>,
}

/// Individual check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    /// Check status
    pub status: HealthStatus,
    /// Check message
    pub message: String,
    /// Check duration
    pub duration: Duration,
}

/// Health monitor
pub struct HealthMonitor {
    metrics_collector: Arc<MetricsCollector>,
}

impl HealthMonitor {
    /// Create a new health monitor
    pub fn new(metrics_collector: Arc<MetricsCollector>) -> Self {
        Self { metrics_collector }
    }
    
    /// Perform a health check
    pub fn check_health(&self) -> HealthCheck {
        let start = Instant::now();
        let mut checks = HashMap::new();
        let mut overall_status = HealthStatus::Healthy;
        
        // Check metrics collection
        let metrics_check = self.check_metrics_collection();
        if matches!(metrics_check.status, HealthStatus::Critical) {
            overall_status = HealthStatus::Critical;
        } else if matches!(metrics_check.status, HealthStatus::Warning) && matches!(overall_status, HealthStatus::Healthy) {
            overall_status = HealthStatus::Warning;
        }
        checks.insert("metrics_collection".to_string(), metrics_check);
        
        // Check error rates
        let error_rate_check = self.check_error_rates();
        if matches!(error_rate_check.status, HealthStatus::Critical) {
            overall_status = HealthStatus::Critical;
        } else if matches!(error_rate_check.status, HealthStatus::Warning) && matches!(overall_status, HealthStatus::Healthy) {
            overall_status = HealthStatus::Warning;
        }
        checks.insert("error_rates".to_string(), error_rate_check);
        
        HealthCheck {
            status: overall_status,
            timestamp: start,
            checks,
        }
    }
    
    fn check_metrics_collection(&self) -> CheckResult {
        let start = Instant::now();
        let global_metrics = self.metrics_collector.get_global_metrics();
        let duration = start.elapsed();
        
        if global_metrics.total_executions > 0 {
            CheckResult {
                status: HealthStatus::Healthy,
                message: format!("Metrics collection active with {} total executions", global_metrics.total_executions),
                duration,
            }
        } else {
            CheckResult {
                status: HealthStatus::Warning,
                message: "No executions recorded yet".to_string(),
                duration,
            }
        }
    }
    
    fn check_error_rates(&self) -> CheckResult {
        let start = Instant::now();
        let global_metrics = self.metrics_collector.get_global_metrics();
        let duration = start.elapsed();
        
        if global_metrics.total_executions == 0 {
            return CheckResult {
                status: HealthStatus::Healthy,
                message: "No executions to check".to_string(),
                duration,
            };
        }
        
        let error_rate = (global_metrics.total_failed as f64 / global_metrics.total_executions as f64) * 100.0;
        
        if error_rate > 50.0 {
            CheckResult {
                status: HealthStatus::Critical,
                message: format!("High error rate: {:.1}%", error_rate),
                duration,
            }
        } else if error_rate > 20.0 {
            CheckResult {
                status: HealthStatus::Warning,
                message: format!("Elevated error rate: {:.1}%", error_rate),
                duration,
            }
        } else {
            CheckResult {
                status: HealthStatus::Healthy,
                message: format!("Error rate within normal range: {:.1}%", error_rate),
                duration,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_metrics_collector() {
        let collector = MetricsCollector::new();
        
        let event = ExecutionEvent {
            tool_name: "test_tool".to_string(),
            duration: Duration::from_millis(100),
            success: true,
            error_type: None,
            timestamp: Instant::now(),
        };
        
        collector.record_execution(event);
        
        let metrics = collector.get_tool_metrics("test_tool").unwrap();
        assert_eq!(metrics.total_executions, 1);
        assert_eq!(metrics.successful_executions, 1);
        assert_eq!(metrics.failed_executions, 0);
        
        let global_metrics = collector.get_global_metrics();
        assert_eq!(global_metrics.total_executions, 1);
        assert_eq!(global_metrics.total_successful, 1);
    }
    
    #[test]
    fn test_health_monitor() {
        let collector = Arc::new(MetricsCollector::new());
        let monitor = HealthMonitor::new(collector.clone());
        
        let health = monitor.check_health();
        assert!(matches!(health.status, HealthStatus::Healthy | HealthStatus::Warning));
        assert!(health.checks.contains_key("metrics_collection"));
        assert!(health.checks.contains_key("error_rates"));
    }
}
