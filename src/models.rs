use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ServiceStatus {
    Running,
    Stopped,
    Unknown,
}

#[derive(Debug, Deserialize)]
pub struct RunningModel {
    pub model: String,
    pub state: String,
}

#[derive(Debug, Deserialize)]
pub struct RunningResponse {
    pub running: Vec<RunningModel>,
}

#[derive(Debug)]
pub struct MetricsResponse {
    pub running_models: Vec<RunningModel>,
    pub total_memory_bytes: u64,
    pub model_count: usize,
}

#[derive(Debug, Clone)]
pub struct Metrics {
    pub prompt_tokens_per_sec: f64,    // llamacpp:prompt_tokens_seconds
    pub predicted_tokens_per_sec: f64, // llamacpp:predicted_tokens_seconds
    pub requests_processing: u32,      // llamacpp:requests_processing
    pub requests_deferred: u32,        // llamacpp:requests_deferred  
    pub kv_cache_usage_ratio: f64,     // llamacpp:kv_cache_usage_ratio (0.0-1.0)
    pub kv_cache_tokens: u32,          // llamacpp:kv_cache_tokens
    pub n_decode_total: u32,           // llamacpp:n_decode_total
    pub memory_mb: f64,                // From sysinfo
}

impl Metrics {
    /// Validate and sanitize metrics
    pub fn validate(&mut self) -> crate::Result<()> {
        // Prompt tokens per second validation
        if self.prompt_tokens_per_sec < 0.0 {
            self.prompt_tokens_per_sec = 0.0;
        } else if self.prompt_tokens_per_sec > 10000.0 {
            return Err("Prompt tokens per second unreasonably high".into());
        }
        
        // Predicted tokens per second validation
        if self.predicted_tokens_per_sec < 0.0 {
            self.predicted_tokens_per_sec = 0.0;
        } else if self.predicted_tokens_per_sec > 1000.0 {
            return Err("Predicted tokens per second unreasonably high".into());
        }
        
        // KV cache usage ratio validation (0.0-1.0)
        if self.kv_cache_usage_ratio < 0.0 {
            self.kv_cache_usage_ratio = 0.0;
        } else if self.kv_cache_usage_ratio > 1.0 {
            self.kv_cache_usage_ratio = 1.0;
        }
        
        // Memory validation
        if self.memory_mb < 0.0 {
            self.memory_mb = 0.0;
        } else if self.memory_mb > 1_000_000.0 { // 1TB
            return Err("Memory value unreasonably high".into());
        }
        
        Ok(())
    }
    
    /// Get KV cache usage as percentage (0-100)
    pub fn kv_cache_percent(&self) -> f64 {
        self.kv_cache_usage_ratio * 100.0
    }
    
    /// Get queue status description
    pub fn queue_status(&self) -> String {
        let total_requests = self.requests_processing + self.requests_deferred;
        
        if total_requests == 0 {
            "idle".to_string()
        } else if self.requests_deferred == 0 {
            if self.requests_processing == 1 {
                "busy".to_string()
            } else {
                format!("busy ({})", self.requests_processing)
            }
        } else {
            format!("queued ({})", total_requests)
        }
    }
}

impl From<MetricsResponse> for Metrics {
    fn from(resp: MetricsResponse) -> Self {
        Self {
            prompt_tokens_per_sec: 0.0, // Will be populated from Prometheus
            predicted_tokens_per_sec: 0.0, // Will be populated from Prometheus
            requests_processing: 0, // Will be populated from Prometheus
            requests_deferred: 0, // Will be populated from Prometheus
            kv_cache_usage_ratio: 0.0, // Will be populated from Prometheus
            kv_cache_tokens: 0, // Will be populated from Prometheus
            n_decode_total: 0, // Will be populated from Prometheus
            memory_mb: resp.total_memory_bytes as f64 / 1_048_576.0, // Convert to MB
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimestampedValue {
    pub timestamp: u64, // Unix timestamp
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsHistory {
    pub tps: VecDeque<TimestampedValue>, // tokens per second (generation)
    pub prompt_tps: VecDeque<TimestampedValue>, // prompt processing speed
    pub memory_mb: VecDeque<TimestampedValue>,
    pub kv_cache_percent: VecDeque<TimestampedValue>, // KV cache usage percentage
    
    #[serde(skip)]
    pub max_size: usize,
}

impl Default for MetricsHistory {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Trend {
    Increasing,
    Decreasing, 
    Stable,
    Insufficient, // Less than 3 data points
}

impl Trend {
    pub fn as_arrow(&self) -> &'static str {
        match self {
            Trend::Increasing => "▲",
            Trend::Decreasing => "▼", 
            Trend::Stable => "▶",
            Trend::Insufficient => "",
        }
    }
    
    pub fn color(&self) -> &'static str {
        match self {
            Trend::Increasing => "#00C853", // Green
            Trend::Decreasing => "#FF1744", // Red
            Trend::Stable => "#666666",     // Gray
            Trend::Insufficient => "#666666",
        }
    }
}

#[derive(Debug)]
pub struct MetricInsights {
    pub current: f64,
    pub min: f64,
    pub max: f64,
    pub trend: Trend,
    pub data_points: usize,
}

impl MetricInsights {    
    pub fn time_context(&self, oldest_timestamp: u64, newest_timestamp: u64) -> String {
        if self.data_points == 0 {
            String::new()
        } else if self.data_points == 1 {
            "(now)".to_string()
        } else {
            let duration_secs = newest_timestamp.saturating_sub(oldest_timestamp);
            let time_text = if duration_secs < 60 {
                format!("{}s", duration_secs)
            } else if duration_secs < 3600 {
                format!("{}m", duration_secs / 60)
            } else {
                format!("{}h", duration_secs / 3600)
            };
            format!("({})", time_text)
        }
    }
}

impl MetricsHistory {
    pub fn new() -> Self {
        Self::with_capacity(crate::constants::HISTORY_SIZE)
    }
    
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            tps: VecDeque::with_capacity(capacity),
            prompt_tps: VecDeque::with_capacity(capacity),
            memory_mb: VecDeque::with_capacity(capacity),
            kv_cache_percent: VecDeque::with_capacity(capacity),
            max_size: capacity,
        }
    }
    
    pub fn push(&mut self, metrics: &Metrics) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Add new values with timestamps
        Self::push_value(&mut self.tps, metrics.predicted_tokens_per_sec, timestamp, self.max_size);
        Self::push_value(&mut self.prompt_tps, metrics.prompt_tokens_per_sec, timestamp, self.max_size);
        Self::push_value(&mut self.memory_mb, metrics.memory_mb, timestamp, self.max_size);
        Self::push_value(&mut self.kv_cache_percent, metrics.kv_cache_percent(), timestamp, self.max_size);
        
        // Clean old data
        self.trim_old_data();
    }
    
    fn push_value(deque: &mut VecDeque<TimestampedValue>, value: f64, timestamp: u64, max_size: usize) {
        deque.push_back(TimestampedValue { timestamp, value });
        
        // Maintain size limit
        while deque.len() > max_size {
            deque.pop_front();
        }
    }
    
    /// Remove data older than the retention window
    pub fn trim_old_data(&mut self) {
        let cutoff = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .saturating_sub(300); // 5 minutes
        
        Self::trim_deque(&mut self.tps, cutoff);
        Self::trim_deque(&mut self.prompt_tps, cutoff);
        Self::trim_deque(&mut self.memory_mb, cutoff);
        Self::trim_deque(&mut self.kv_cache_percent, cutoff);
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
        self.prompt_tps.clear();
        self.memory_mb.clear();
        self.kv_cache_percent.clear();
    }
    
    /// Get comprehensive insights for a metric (high-performance, in-memory)
    pub fn get_insights(&self, deque: &VecDeque<TimestampedValue>) -> MetricInsights {
        let data_points = deque.len();
        
        if data_points == 0 {
            return MetricInsights {
                current: 0.0,
                min: 0.0,
                max: 0.0,
                trend: Trend::Insufficient,
                data_points: 0,
            };
        }
        
        let current = deque.back().unwrap().value;
        
        // Fast single-pass min/max calculation
        let (min, max) = if data_points == 1 {
            (current, current)
        } else {
            deque.iter().map(|tv| tv.value).fold(
                (f64::INFINITY, f64::NEG_INFINITY),
                |(min_acc, max_acc), val| (min_acc.min(val), max_acc.max(val))
            )
        };
        
        // Calculate trend from last 3-5 points (or all if fewer)
        let trend = self.calculate_trend(deque);
        
        MetricInsights {
            current,
            min,
            max,
            trend,
            data_points,
        }
    }
    
    /// High-performance trend calculation using last few points
    fn calculate_trend(&self, deque: &VecDeque<TimestampedValue>) -> Trend {
        let len = deque.len();
        if len < 3 {
            return Trend::Insufficient;
        }
        
        // Use last 3-5 points for trend calculation
        let sample_size = (len.min(5)).max(3);
        let start_idx = len - sample_size;
        
        let points: Vec<f64> = deque.iter()
            .skip(start_idx)
            .map(|tv| tv.value)
            .collect();
            
        // Simple slope calculation: (last - first) / distance
        let first = points[0];
        let last = points[points.len() - 1];
        let slope = (last - first) / (points.len() - 1) as f64;
        
        // Threshold-based trend detection
        let threshold = (last + first) * 0.02; // 2% change threshold
        
        if slope.abs() < threshold {
            Trend::Stable
        } else if slope > 0.0 {
            Trend::Increasing
        } else {
            Trend::Decreasing
        }
    }
    
    /// Fast anomaly detection using recent average
    fn detect_anomaly(&self, deque: &VecDeque<TimestampedValue>, current: f64) -> bool {
        let len = deque.len();
        if len < 5 {
            return false; // Need history to detect anomalies
        }
        
        // Calculate average of recent points (excluding current)
        let recent_count = (len - 1).min(10); // Last 10 points excluding current
        let start_idx = len - 1 - recent_count;
        
        let recent_sum: f64 = deque.iter()
            .skip(start_idx)
            .take(recent_count)
            .map(|tv| tv.value)
            .sum();
        let recent_avg = recent_sum / recent_count as f64;
        
        // Anomaly if current is >150% or <50% of recent average
        current > recent_avg * 1.5 || current < recent_avg * 0.5
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