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
pub struct ModelMetrics {
    pub model_name: String,
    pub metrics: Metrics,
}

#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub cpu_usage_percent: f64,
    pub total_memory_gb: f64,
    pub used_memory_gb: f64,
    pub available_memory_gb: f64,
    pub memory_usage_percent: f64,
    pub gpu_usage_percent: Option<f64>, // None if GPU not available or detectable
    pub gpu_memory_used_gb: Option<f64>,
    pub gpu_memory_total_gb: Option<f64>,
    pub load_average_1m: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct AllModelMetrics {
    pub models: Vec<ModelMetrics>,
    pub total_llama_memory_mb: f64, // Memory used by llama processes specifically
    pub system_metrics: SystemMetrics,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetricsHistory {
    pub model_name: String,
    pub history: MetricsHistory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllModelMetricsHistory {
    pub models: std::collections::HashMap<String, MetricsHistory>,
    pub total_llama_memory_mb: VecDeque<TimestampedValue>,
    
    // System metrics history
    pub cpu_usage_percent: VecDeque<TimestampedValue>,
    pub memory_usage_percent: VecDeque<TimestampedValue>,
    pub used_memory_gb: VecDeque<TimestampedValue>,
    pub gpu_usage_percent: VecDeque<TimestampedValue>,
    pub gpu_memory_used_gb: VecDeque<TimestampedValue>,
    pub load_average_1m: VecDeque<TimestampedValue>,
    
    #[serde(skip)]
    pub max_size: usize,
}

impl Default for AllModelMetricsHistory {
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

impl AllModelMetricsHistory {
    pub fn new() -> Self {
        Self::with_capacity(crate::constants::HISTORY_SIZE)
    }
    
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            models: std::collections::HashMap::new(),
            total_llama_memory_mb: VecDeque::with_capacity(capacity),
            cpu_usage_percent: VecDeque::with_capacity(capacity),
            memory_usage_percent: VecDeque::with_capacity(capacity),
            used_memory_gb: VecDeque::with_capacity(capacity),
            gpu_usage_percent: VecDeque::with_capacity(capacity),
            gpu_memory_used_gb: VecDeque::with_capacity(capacity),
            load_average_1m: VecDeque::with_capacity(capacity),
            max_size: capacity,
        }
    }
    
    pub fn push(&mut self, all_metrics: &AllModelMetrics) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Add llama-specific memory
        Self::push_value(&mut self.total_llama_memory_mb, all_metrics.total_llama_memory_mb, timestamp, self.max_size);
        
        // Add system metrics
        let sys = &all_metrics.system_metrics;
        Self::push_value(&mut self.cpu_usage_percent, sys.cpu_usage_percent, timestamp, self.max_size);
        Self::push_value(&mut self.memory_usage_percent, sys.memory_usage_percent, timestamp, self.max_size);
        Self::push_value(&mut self.used_memory_gb, sys.used_memory_gb, timestamp, self.max_size);
        Self::push_value(&mut self.load_average_1m, sys.load_average_1m.unwrap_or(0.0), timestamp, self.max_size);
        
        // GPU metrics removed - powermetrics was too expensive and unreliable
        
        // Add per-model metrics
        for model_metrics in &all_metrics.models {
            let history = self.models.entry(model_metrics.model_name.clone())
                .or_insert_with(|| MetricsHistory::with_capacity(self.max_size));
            history.push(&model_metrics.metrics);
        }
        
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
        
        Self::trim_deque(&mut self.total_llama_memory_mb, cutoff);
        Self::trim_deque(&mut self.cpu_usage_percent, cutoff);
        Self::trim_deque(&mut self.memory_usage_percent, cutoff);
        Self::trim_deque(&mut self.used_memory_gb, cutoff);
        Self::trim_deque(&mut self.gpu_usage_percent, cutoff);
        Self::trim_deque(&mut self.gpu_memory_used_gb, cutoff);
        Self::trim_deque(&mut self.load_average_1m, cutoff);
        
        // Trim all model histories
        for (_, history) in self.models.iter_mut() {
            history.trim_old_data();
        }
        
        // Remove models that have no recent data
        self.models.retain(|_, history| !history.tps.is_empty());
    }
    
    fn trim_deque(deque: &mut VecDeque<TimestampedValue>, cutoff: u64) {
        while deque.front().map_or(false, |v| v.timestamp < cutoff) {
            deque.pop_front();
        }
    }
    
    pub fn clear(&mut self) {
        self.models.clear();
        self.total_llama_memory_mb.clear();
        self.cpu_usage_percent.clear();
        self.memory_usage_percent.clear();
        self.used_memory_gb.clear();
        self.gpu_usage_percent.clear();
        self.gpu_memory_used_gb.clear();
        self.load_average_1m.clear();
    }
    
    pub fn get_model_history(&self, model_name: &str) -> Option<&MetricsHistory> {
        self.models.get(model_name)
    }
    
    pub fn get_model_names(&self) -> Vec<String> {
        self.models.keys().cloned().collect()
    }
    
    /// Get insights for system CPU usage
    pub fn get_cpu_insights(&self) -> MetricInsights {
        self.get_insights_for_data(&self.cpu_usage_percent)
    }
    
    /// Get insights for system memory usage
    pub fn get_system_memory_insights(&self) -> MetricInsights {
        self.get_insights_for_data(&self.memory_usage_percent)
    }
    
    // GPU insights removed - powermetrics was too expensive and unreliable
    
    /// Get insights for load average
    pub fn get_load_insights(&self) -> MetricInsights {
        self.get_insights_for_data(&self.load_average_1m)
    }
    
    /// Helper method to get insights for any data series
    fn get_insights_for_data(&self, data: &VecDeque<TimestampedValue>) -> MetricInsights {
        let data_points = data.len();
        
        if data_points == 0 {
            return MetricInsights {
                current: 0.0,
                min: 0.0,
                max: 0.0,
                trend: Trend::Insufficient,
                data_points: 0,
            };
        }
        
        let current = data.back().unwrap().value;
        
        // Fast single-pass min/max calculation
        let (min, max) = if data_points == 1 {
            (current, current)
        } else {
            data.iter().map(|tv| tv.value).fold(
                (f64::INFINITY, f64::NEG_INFINITY),
                |(min_acc, max_acc), val| (min_acc.min(val), max_acc.max(val))
            )
        };
        
        // Calculate trend 
        let trend = self.calculate_trend_for_data(data);
        
        MetricInsights {
            current,
            min,
            max,
            trend,
            data_points,
        }
    }
    
    /// Calculate trend for any data series
    fn calculate_trend_for_data(&self, data: &VecDeque<TimestampedValue>) -> Trend {
        let len = data.len();
        if len < 3 {
            return Trend::Insufficient;
        }
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Prefer 30-second window, fall back to 15-second, then minimum 3 points
        let preferred_windows = [30u64, 15u64];
        
        for &window_secs in &preferred_windows {
            let cutoff_time = now.saturating_sub(window_secs);
            let window_points: Vec<&TimestampedValue> = data.iter()
                .filter(|tv| tv.timestamp >= cutoff_time)
                .collect();
            
            if window_points.len() >= 3 {
                return self.calculate_trend_from_points_generic(&window_points, window_secs);
            }
        }
        
        // Fall back to using all available points if we have at least 3
        let all_points: Vec<&TimestampedValue> = data.iter().collect();
        if all_points.len() >= 3 {
            let time_span = all_points.last().unwrap().timestamp 
                .saturating_sub(all_points.first().unwrap().timestamp);
            return self.calculate_trend_from_points_generic(&all_points, time_span.max(1));
        }
        
        Trend::Insufficient
    }
    
    fn calculate_trend_from_points_generic(&self, points: &[&TimestampedValue], time_span_secs: u64) -> Trend {
        if points.len() < 3 {
            return Trend::Insufficient;
        }
        
        let values: Vec<f64> = points.iter().map(|tv| tv.value).collect();
        let first = values[0];
        let last = values[values.len() - 1];
        
        // Check if all values are essentially the same (flat line)
        let max_val = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let min_val = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let value_range = max_val - min_val;
        
        // If range is tiny, it's definitely stable regardless of slope calculation
        if value_range < f64::EPSILON * 10.0 {
            return Trend::Stable;
        }
        
        // Calculate slope over actual time (value change per second)
        let time_span = time_span_secs.max(1) as f64;
        let slope = (last - first) / time_span;
        
        // Adaptive thresholds based on value magnitude and time span
        let value_magnitude = (first.abs() + last.abs()) / 2.0;
        
        // For trends, we want meaningful change over the time period
        let relative_threshold = value_magnitude * 0.05 / time_span; // 5% change over time period
        let absolute_threshold = 0.01 / time_span; // Minimum absolute change per second
        let threshold = relative_threshold.max(absolute_threshold);
        
        // Require longer time spans for more sensitive detection
        let time_factor = if time_span >= 30.0 { 1.0 } else { 1.5 }; // More conservative for short spans
        let adjusted_threshold = threshold * time_factor;
        
        if slope.abs() < adjusted_threshold {
            Trend::Stable
        } else if slope > 0.0 {
            Trend::Increasing
        } else {
            Trend::Decreasing
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
    
    /// Time-based trend calculation using 15-30 second window when available
    fn calculate_trend(&self, deque: &VecDeque<TimestampedValue>) -> Trend {
        let len = deque.len();
        if len < 3 {
            return Trend::Insufficient;
        }
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Prefer 30-second window, fall back to 15-second, then minimum 3 points
        let preferred_windows = [30u64, 15u64];
        
        for &window_secs in &preferred_windows {
            let cutoff_time = now.saturating_sub(window_secs);
            let window_points: Vec<&TimestampedValue> = deque.iter()
                .filter(|tv| tv.timestamp >= cutoff_time)
                .collect();
            
            if window_points.len() >= 3 {
                return self.calculate_trend_from_points(&window_points, window_secs);
            }
        }
        
        // Fall back to using all available points if we have at least 3
        let all_points: Vec<&TimestampedValue> = deque.iter().collect();
        if all_points.len() >= 3 {
            let time_span = all_points.last().unwrap().timestamp 
                .saturating_sub(all_points.first().unwrap().timestamp);
            return self.calculate_trend_from_points(&all_points, time_span.max(1));
        }
        
        Trend::Insufficient
    }
    
    fn calculate_trend_from_points(&self, points: &[&TimestampedValue], time_span_secs: u64) -> Trend {
        if points.len() < 3 {
            return Trend::Insufficient;
        }
        
        let values: Vec<f64> = points.iter().map(|tv| tv.value).collect();
        let first = values[0];
        let last = values[values.len() - 1];
        
        // Check if all values are essentially the same (flat line)
        let max_val = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let min_val = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let value_range = max_val - min_val;
        
        // If range is tiny, it's definitely stable regardless of slope calculation
        if value_range < f64::EPSILON * 10.0 {
            return Trend::Stable;
        }
        
        // Calculate slope over actual time (value change per second)
        let time_span = time_span_secs.max(1) as f64;
        let slope = (last - first) / time_span;
        
        // Adaptive thresholds based on value magnitude and time span
        let value_magnitude = (first.abs() + last.abs()) / 2.0;
        
        // For trends, we want meaningful change over the time period
        let relative_threshold = value_magnitude * 0.05 / time_span; // 5% change over time period
        let absolute_threshold = 0.01 / time_span; // Minimum absolute change per second
        let threshold = relative_threshold.max(absolute_threshold);
        
        // Require longer time spans for more sensitive detection
        let time_factor = if time_span >= 30.0 { 1.0 } else { 1.5 }; // More conservative for short spans
        let adjusted_threshold = threshold * time_factor;
        
        if slope.abs() < adjusted_threshold {
            Trend::Stable
        } else if slope > 0.0 {
            Trend::Increasing
        } else {
            Trend::Decreasing
        }
    }
    
    /// Fast anomaly detection using recent average
    #[allow(dead_code)]
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

impl AllModelMetricsHistory {
    /// Calculate statistics for total llama memory
    pub fn calculate_memory_stats(&self) -> MetricStats {
        if self.total_llama_memory_mb.is_empty() {
            return MetricStats::default();
        }
        
        let values: Vec<f64> = self.total_llama_memory_mb.iter().map(|tv| tv.value).collect();
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
    
    /// Get insights for total llama memory
    pub fn get_memory_insights(&self) -> MetricInsights {
        let data_points = self.total_llama_memory_mb.len();
        
        if data_points == 0 {
            return MetricInsights {
                current: 0.0,
                min: 0.0,
                max: 0.0,
                trend: Trend::Insufficient,
                data_points: 0,
            };
        }
        
        let current = self.total_llama_memory_mb.back().unwrap().value;
        
        // Fast single-pass min/max calculation
        let (min, max) = if data_points == 1 {
            (current, current)
        } else {
            self.total_llama_memory_mb.iter().map(|tv| tv.value).fold(
                (f64::INFINITY, f64::NEG_INFINITY),
                |(min_acc, max_acc), val| (min_acc.min(val), max_acc.max(val))
            )
        };
        
        // Calculate trend from last 3-5 points (or all if fewer)
        let trend = self.calculate_memory_trend();
        
        MetricInsights {
            current,
            min,
            max,
            trend,
            data_points,
        }
    }
    
    /// Time-based trend calculation for memory using 15-30 second window when available
    fn calculate_memory_trend(&self) -> Trend {
        let len = self.total_llama_memory_mb.len();
        if len < 3 {
            return Trend::Insufficient;
        }
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Prefer 30-second window, fall back to 15-second, then minimum 3 points
        let preferred_windows = [30u64, 15u64];
        
        for &window_secs in &preferred_windows {
            let cutoff_time = now.saturating_sub(window_secs);
            let window_points: Vec<&TimestampedValue> = self.total_llama_memory_mb.iter()
                .filter(|tv| tv.timestamp >= cutoff_time)
                .collect();
            
            if window_points.len() >= 3 {
                return self.calculate_memory_trend_from_points(&window_points, window_secs);
            }
        }
        
        // Fall back to using all available points if we have at least 3
        let all_points: Vec<&TimestampedValue> = self.total_llama_memory_mb.iter().collect();
        if all_points.len() >= 3 {
            let time_span = all_points.last().unwrap().timestamp 
                .saturating_sub(all_points.first().unwrap().timestamp);
            return self.calculate_memory_trend_from_points(&all_points, time_span.max(1));
        }
        
        Trend::Insufficient
    }
    
    fn calculate_memory_trend_from_points(&self, points: &[&TimestampedValue], time_span_secs: u64) -> Trend {
        if points.len() < 3 {
            return Trend::Insufficient;
        }
        
        let values: Vec<f64> = points.iter().map(|tv| tv.value).collect();
        let first = values[0];
        let last = values[values.len() - 1];
        
        // Check if all values are essentially the same (flat line)
        let max_val = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let min_val = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let value_range = max_val - min_val;
        
        // If range is tiny, it's definitely stable regardless of slope calculation
        if value_range < f64::EPSILON * 10.0 {
            return Trend::Stable;
        }
        
        // Calculate slope over actual time (value change per second)
        let time_span = time_span_secs.max(1) as f64;
        let slope = (last - first) / time_span;
        
        // Adaptive thresholds based on value magnitude and time span
        let value_magnitude = (first.abs() + last.abs()) / 2.0;
        
        // For trends, we want meaningful change over the time period
        let relative_threshold = value_magnitude * 0.05 / time_span; // 5% change over time period
        let absolute_threshold = 0.01 / time_span; // Minimum absolute change per second
        let threshold = relative_threshold.max(absolute_threshold);
        
        // Require longer time spans for more sensitive detection
        let time_factor = if time_span >= 30.0 { 1.0 } else { 1.5 }; // More conservative for short spans
        let adjusted_threshold = threshold * time_factor;
        
        if slope.abs() < adjusted_threshold {
            Trend::Stable
        } else if slope > 0.0 {
            Trend::Increasing
        } else {
            Trend::Decreasing
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