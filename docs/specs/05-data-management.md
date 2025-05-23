# Phase 5: Data Management Specification

## Overview

This phase implements the data management layer, including metrics history storage, efficient circular buffers, and data persistence strategies for maintaining state across plugin restarts.

## Goals

- Implement efficient circular buffer for metrics history
- Add data validation and sanitization
- Create persistence layer for maintaining history
- Optimize memory usage for long-running operation

## Implementation

### 5.1 Enhanced Metrics History

Enhance the MetricsHistory structure in src/models.rs:

```rust
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize)]
pub struct TimestampedValue {
    pub timestamp: u64, // Unix timestamp
    pub value: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricsHistory {
    pub tps: VecDeque<TimestampedValue>,
    pub memory_mb: VecDeque<TimestampedValue>,
    pub cache_hit_rate: VecDeque<TimestampedValue>,
    
    #[serde(skip)]
    max_size: usize,
}

impl MetricsHistory {
    pub fn new() -> Self {
        Self::with_capacity(crate::constants::HISTORY_SIZE)
    }
    
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            tps: VecDeque::with_capacity(capacity),
            memory_mb: VecDeque::with_capacity(capacity),
            cache_hit_rate: VecDeque::with_capacity(capacity),
            max_size: capacity,
        }
    }
    
    pub fn push(&mut self, metrics: &Metrics) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Add new values with timestamps
        self.push_value(&mut self.tps, metrics.tps, timestamp);
        self.push_value(&mut self.memory_mb, metrics.memory_mb, timestamp);
        self.push_value(&mut self.cache_hit_rate, metrics.cache_hit_rate, timestamp);
        
        // Clean old data
        self.trim_old_data();
    }
    
    fn push_value(&self, deque: &mut VecDeque<TimestampedValue>, value: f64, timestamp: u64) {
        deque.push_back(TimestampedValue { timestamp, value });
        
        // Maintain size limit
        while deque.len() > self.max_size {
            deque.pop_front();
        }
    }
    
    /// Remove data older than the retention window
    fn trim_old_data(&mut self) {
        let cutoff = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .saturating_sub(300); // 5 minutes
        
        Self::trim_deque(&mut self.tps, cutoff);
        Self::trim_deque(&mut self.memory_mb, cutoff);
        Self::trim_deque(&mut self.cache_hit_rate, cutoff);
    }
    
    fn trim_deque(deque: &mut VecDeque<TimestampedValue>, cutoff: u64) {
        while deque.front().map_or(false, |v| v.timestamp < cutoff) {
            deque.pop_front();
        }
    }
    
    /// Get values as simple vector for charting
    pub fn get_values(&self, deque: &VecDeque<TimestampedValue>) -> VecDeque<f64> {
        deque.iter().map(|tv| tv.value).collect()
    }
    
    pub fn clear(&mut self) {
        self.tps.clear();
        self.memory_mb.clear();
        self.cache_hit_rate.clear();
    }
    
    /// Calculate statistics for a metric
    pub fn calculate_stats(&self, deque: &VecDeque<TimestampedValue>) -> MetricStats {
        if deque.is_empty() {
            return MetricStats::default();
        }
        
        let values: Vec<f64> = deque.iter().map(|tv| tv.value).collect();
        let sum: f64 = values.iter().sum();
        let count = values.len() as f64;
        let mean = sum / count;
        
        let min = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        
        let variance = values.iter()
            .map(|&v| (v - mean).powi(2))
            .sum::<f64>() / count;
        let std_dev = variance.sqrt();
        
        MetricStats {
            mean,
            min,
            max,
            std_dev,
            count: values.len(),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MetricStats {
    pub mean: f64,
    pub min: f64,
    pub max: f64,
    pub std_dev: f64,
    pub count: usize,
}
```

### 5.2 Data Persistence

Create src/persistence.rs for saving/loading history:

```rust
use std::fs;
use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};
use crate::models::MetricsHistory;

const PERSISTENCE_FILE: &str = "llama-swap-metrics.json";

/// Get the path for persistence file
fn get_persistence_path() -> crate::Result<PathBuf> {
    let home = std::env::var("HOME")
        .map_err(|_| "Failed to get HOME directory")?;
    
    let data_dir = Path::new(&home)
        .join("Library")
        .join("Application Support")
        .join("SwiftBar")
        .join("PluginData");
    
    // Create directory if it doesn't exist
    fs::create_dir_all(&data_dir)
        .map_err(|e| format!("Failed to create data directory: {}", e))?;
    
    Ok(data_dir.join(PERSISTENCE_FILE))
}

/// Save metrics history to disk
pub fn save_metrics(history: &MetricsHistory) -> crate::Result<()> {
    let path = get_persistence_path()?;
    
    let json = serde_json::to_string_pretty(history)
        .map_err(|e| format!("Failed to serialize metrics: {}", e))?;
    
    fs::write(&path, json)
        .map_err(|e| format!("Failed to write metrics file: {}", e))?;
    
    Ok(())
}

/// Load metrics history from disk
pub fn load_metrics() -> crate::Result<MetricsHistory> {
    let path = get_persistence_path()?;
    
    if !path.exists() {
        // No saved data, return empty history
        return Ok(MetricsHistory::new());
    }
    
    let json = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read metrics file: {}", e))?;
    
    let mut history: MetricsHistory = serde_json::from_str(&json)
        .map_err(|e| format!("Failed to parse metrics file: {}", e))?;
    
    // Set max_size since it's not serialized
    history.max_size = crate::constants::HISTORY_SIZE;
    
    // Trim any old data
    history.trim_old_data();
    
    Ok(history)
}

/// Delete persistence file
pub fn clear_persistence() -> crate::Result<()> {
    let path = get_persistence_path()?;
    
    if path.exists() {
        fs::remove_file(&path)
            .map_err(|e| format!("Failed to delete metrics file: {}", e))?;
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Metrics, TimestampedValue};
    
    #[test]
    fn test_save_load_cycle() {
        let mut history = MetricsHistory::new();
        
        // Add some test data
        let metrics = Metrics {
            tps: 42.0,
            memory_mb: 1024.0,
            cache_hit_rate: 95.0,
        };
        
        history.push(&metrics);
        
        // Save
        assert!(save_metrics(&history).is_ok());
        
        // Load
        let loaded = load_metrics().unwrap();
        assert_eq!(loaded.tps.len(), 1);
        assert_eq!(loaded.tps[0].value, 42.0);
        
        // Cleanup
        let _ = clear_persistence();
    }
}
```

### 5.3 Memory-Efficient Data Structures

Create optimized data structures for minimal memory usage:

```rust
// In src/models.rs, add:

/// Compact representation of metric history using fixed-point arithmetic
#[derive(Debug)]
pub struct CompactMetricsHistory {
    /// Timestamps as seconds since a recent epoch (saves 4 bytes per timestamp)
    base_timestamp: u64,
    timestamps: Vec<u32>, // Offsets from base_timestamp
    
    /// Values stored as fixed-point integers (2 decimal places)
    tps_values: Vec<u16>,        // 0-655.35 TPS
    memory_mb_values: Vec<u32>,   // 0-42949672.95 MB
    cache_rate_values: Vec<u8>,  // 0-100%
}

impl CompactMetricsHistory {
    pub fn from_history(history: &MetricsHistory) -> Self {
        let base_timestamp = history.tps.front()
            .map(|tv| tv.timestamp)
            .unwrap_or_else(|| SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
        
        let mut compact = Self {
            base_timestamp,
            timestamps: Vec::new(),
            tps_values: Vec::new(),
            memory_mb_values: Vec::new(),
            cache_rate_values: Vec::new(),
        };
        
        // Convert TPS
        for tv in &history.tps {
            let offset = (tv.timestamp - base_timestamp) as u32;
            let value = (tv.value * 100.0).round() as u16;
            compact.timestamps.push(offset);
            compact.tps_values.push(value);
        }
        
        // Convert memory (assuming same timestamps)
        for tv in &history.memory_mb {
            let value = (tv.value * 100.0).round() as u32;
            compact.memory_mb_values.push(value);
        }
        
        // Convert cache rate
        for tv in &history.cache_hit_rate {
            let value = tv.value.round() as u8;
            compact.cache_rate_values.push(value);
        }
        
        compact
    }
    
    pub fn memory_usage(&self) -> usize {
        std::mem::size_of::<Self>() +
        self.timestamps.capacity() * std::mem::size_of::<u32>() +
        self.tps_values.capacity() * std::mem::size_of::<u16>() +
        self.memory_mb_values.capacity() * std::mem::size_of::<u32>() +
        self.cache_rate_values.capacity() * std::mem::size_of::<u8>()
    }
}
```

### 5.4 Data Validation

Add validation to ensure data quality:

```rust
// In src/models.rs, add:

impl Metrics {
    /// Validate and sanitize metrics
    pub fn validate(&mut self) -> crate::Result<()> {
        // TPS validation
        if self.tps < 0.0 {
            self.tps = 0.0;
        } else if self.tps > 10000.0 {
            return Err("TPS value unreasonably high".into());
        }
        
        // Memory validation
        if self.memory_mb < 0.0 {
            self.memory_mb = 0.0;
        } else if self.memory_mb > 1_000_000.0 { // 1TB
            return Err("Memory value unreasonably high".into());
        }
        
        // Cache hit rate validation
        self.cache_hit_rate = self.cache_hit_rate.clamp(0.0, 100.0);
        
        Ok(())
    }
}

// Update metrics fetching to validate:
pub fn fetch_metrics(client: &Client) -> crate::Result<Metrics> {
    // ... existing fetch code ...
    
    let mut metrics: Metrics = metrics_response.into();
    metrics.validate()?;
    
    Ok(metrics)
}
```

### 5.5 Integration with Main Loop

Update src/main.rs to use persistence:

```rust
mod persistence;

struct PluginState {
    http_client: Client,
    metrics_history: MetricsHistory,
    current_status: ServiceStatus,
    is_first_iteration: bool,
    last_save_time: std::time::Instant,
}

impl PluginState {
    fn new() -> Result<Self> {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(constants::API_TIMEOUT_SECS))
            .build()?;
        
        // Try to load existing history
        let metrics_history = persistence::load_metrics()
            .unwrap_or_else(|e| {
                eprintln!("Failed to load metrics history: {}", e);
                MetricsHistory::new()
            });
        
        Ok(Self {
            http_client,
            metrics_history,
            current_status: ServiceStatus::Unknown,
            is_first_iteration: true,
            last_save_time: std::time::Instant::now(),
        })
    }
    
    fn save_if_needed(&mut self) {
        // Save every 30 seconds
        if self.last_save_time.elapsed() > Duration::from_secs(30) {
            if let Err(e) = persistence::save_metrics(&self.metrics_history) {
                eprintln!("Failed to save metrics: {}", e);
            } else {
                self.last_save_time = std::time::Instant::now();
            }
        }
    }
}

// In the main loop:
fn run_streaming_mode() -> Result<()> {
    let mut state = PluginState::new()?;
    
    // ... existing loop code ...
    
    // After update_state:
    state.save_if_needed();
    
    // ... rest of loop ...
    
    // On shutdown:
    if let Err(e) = persistence::save_metrics(&state.metrics_history) {
        eprintln!("Failed to save final metrics: {}", e);
    }
}
```

### 5.6 Advanced Features

Add rolling statistics window:

```rust
// In src/models.rs:

impl MetricsHistory {
    /// Get metrics for a specific time window
    pub fn get_window(&self, seconds: u64) -> MetricsWindow {
        let cutoff = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .saturating_sub(seconds);
        
        MetricsWindow {
            tps: self.filter_by_time(&self.tps, cutoff),
            memory_mb: self.filter_by_time(&self.memory_mb, cutoff),
            cache_hit_rate: self.filter_by_time(&self.cache_hit_rate, cutoff),
        }
    }
    
    fn filter_by_time(&self, deque: &VecDeque<TimestampedValue>, cutoff: u64) -> Vec<f64> {
        deque.iter()
            .filter(|tv| tv.timestamp >= cutoff)
            .map(|tv| tv.value)
            .collect()
    }
    
    /// Detect anomalies using simple statistical methods
    pub fn detect_anomalies(&self) -> AnomalyReport {
        let tps_stats = self.calculate_stats(&self.tps);
        let mem_stats = self.calculate_stats(&self.memory_mb);
        
        let mut anomalies = Vec::new();
        
        // Check for TPS anomalies (3 standard deviations)
        if let Some(latest_tps) = self.tps.back() {
            if (latest_tps.value - tps_stats.mean).abs() > 3.0 * tps_stats.std_dev {
                anomalies.push(Anomaly {
                    metric: "TPS".to_string(),
                    value: latest_tps.value,
                    expected_range: (
                        tps_stats.mean - 3.0 * tps_stats.std_dev,
                        tps_stats.mean + 3.0 * tps_stats.std_dev,
                    ),
                });
            }
        }
        
        AnomalyReport { anomalies }
    }
}

#[derive(Debug)]
pub struct MetricsWindow {
    pub tps: Vec<f64>,
    pub memory_mb: Vec<f64>,
    pub cache_hit_rate: Vec<f64>,
}

#[derive(Debug)]
pub struct AnomalyReport {
    pub anomalies: Vec<Anomaly>,
}

#[derive(Debug)]
pub struct Anomaly {
    pub metric: String,
    pub value: f64,
    pub expected_range: (f64, f64),
}
```

## Performance Considerations

### Memory Usage
- Each metric point: ~24 bytes (8 timestamp + 8 value + 8 overhead)
- 60 points × 3 metrics = ~4.3KB active memory
- Compact format reduces to ~1.5KB

### CPU Usage
- Validation: O(1) per metric
- Statistics: O(n) calculated on demand
- Persistence: O(n) every 30 seconds
- Anomaly detection: O(n) optional feature

## Testing Data Management

```rust
// In tests/data_management.rs:

#[test]
fn test_circular_buffer_limits() {
    let mut history = MetricsHistory::with_capacity(10);
    
    // Add more than capacity
    for i in 0..20 {
        let metrics = Metrics {
            tps: i as f64,
            memory_mb: 100.0,
            cache_hit_rate: 90.0,
        };
        history.push(&metrics);
    }
    
    // Should only have last 10
    assert_eq!(history.tps.len(), 10);
    assert_eq!(history.get_values(&history.tps)[0], 10.0);
}

#[test]
fn test_persistence_corruption() {
    // Test handling of corrupted persistence file
    let path = persistence::get_persistence_path().unwrap();
    fs::write(&path, "invalid json").unwrap();
    
    // Should return empty history, not panic
    let history = persistence::load_metrics().unwrap();
    assert_eq!(history.tps.len(), 0);
}
```

## Next Steps

With data management complete, proceed to [Phase 6: Menu Construction](06-menu-construction.md) to build the final user interface.