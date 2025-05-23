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

#[derive(Debug)]
pub struct Metrics {
    pub prompt_tokens_per_sec: f64,    // llamacpp:prompt_tokens_seconds
    pub predicted_tokens_per_sec: f64, // llamacpp:predicted_tokens_seconds
    pub requests_processing: u32,      // llamacpp:requests_processing
    pub memory_mb: f64,               // From sysinfo
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
        
        // Memory validation
        if self.memory_mb < 0.0 {
            self.memory_mb = 0.0;
        } else if self.memory_mb > 1_000_000.0 { // 1TB
            return Err("Memory value unreasonably high".into());
        }
        
        Ok(())
    }
}

impl From<MetricsResponse> for Metrics {
    fn from(resp: MetricsResponse) -> Self {
        Self {
            prompt_tokens_per_sec: 0.0, // Will be populated from Prometheus
            predicted_tokens_per_sec: 0.0, // Will be populated from Prometheus
            requests_processing: 0, // Will be populated from Prometheus
            memory_mb: resp.total_memory_bytes as f64 / 1_048_576.0, // Convert to MB
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TimestampedValue {
    pub timestamp: u64, // Unix timestamp
    pub value: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricsHistory {
    pub tps: VecDeque<TimestampedValue>, // tokens per second (generation)
    pub memory_mb: VecDeque<TimestampedValue>,
    pub cache_hit_rate: VecDeque<TimestampedValue>, // we'll use prompt speed as proxy
    
    #[serde(skip)]
    pub max_size: usize,
}

impl Default for MetricsHistory {
    fn default() -> Self {
        Self::new()
    }
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
        Self::push_value(&mut self.tps, metrics.predicted_tokens_per_sec, timestamp, self.max_size);
        Self::push_value(&mut self.memory_mb, metrics.memory_mb, timestamp, self.max_size);
        // Use prompt speed as a proxy for cache efficiency (higher = better)
        let cache_proxy = if metrics.prompt_tokens_per_sec > 0.0 {
            (metrics.prompt_tokens_per_sec / 1000.0 * 100.0).min(100.0) // Convert to percentage
        } else {
            0.0
        };
        Self::push_value(&mut self.cache_hit_rate, cache_proxy, timestamp, self.max_size);
        
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