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
}

#[derive(Debug, Clone)]
pub struct AllMetrics {
    pub models: Vec<ModelMetrics>,
    pub total_llama_memory_mb: f64,
    pub system_metrics: SystemMetrics,
}

#[derive(Debug, Clone)]
pub struct Metrics {
    pub prompt_tokens_per_sec: f64,
    pub predicted_tokens_per_sec: f64,
    pub requests_processing: u32,
    pub requests_deferred: u32,
    pub kv_cache_usage_ratio: f64,
    pub kv_cache_tokens: u32,
    pub n_decode_total: u32,
    pub memory_mb: f64,
}

impl Metrics {    
    pub fn kv_cache_percent(&self) -> f64 {
        self.kv_cache_usage_ratio * 100.0
    }
    
    pub fn queue_status(&self) -> String {
        let total_requests = self.requests_processing + self.requests_deferred;
        
        match (total_requests, self.requests_deferred) {
            (0, _) => "idle".to_string(),
            (_, 0) if self.requests_processing == 1 => "busy".to_string(),
            (_, 0) => format!("busy ({})", self.requests_processing),
            _ => format!("queued ({})", total_requests),
        }
    }
}

impl From<MetricsResponse> for Metrics {
    fn from(resp: MetricsResponse) -> Self {
        Self {
            prompt_tokens_per_sec: 0.0,
            predicted_tokens_per_sec: 0.0,
            requests_processing: 0,
            requests_deferred: 0,
            kv_cache_usage_ratio: 0.0,
            kv_cache_tokens: 0,
            n_decode_total: 0,
            memory_mb: resp.total_memory_bytes as f64 / 1_048_576.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimestampedValue {
    pub timestamp: u64,
    pub value: f64,
}

#[derive(Debug)]
pub struct MetricInsights {
    pub current: f64,
    pub min: f64,
    pub max: f64,
    pub data_points: usize,
}

impl MetricInsights {
    pub fn time_context(&self, oldest_timestamp: u64, newest_timestamp: u64) -> String {
        match self.data_points {
            0 => String::new(),
            1 => "(now)".to_string(),
            _ => {
                let duration_secs = newest_timestamp.saturating_sub(oldest_timestamp);
                let time_text = match duration_secs {
                    s if s < 60 => format!("{}s", s),
                    s if s < 3600 => format!("{}m", s / 60),
                    s => format!("{}h", s / 3600),
                };
                format!("({})", time_text)
            }
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


// Unified analysis operations
struct DataAnalyzer;

impl DataAnalyzer {
    fn get_insights(data: &VecDeque<TimestampedValue>) -> MetricInsights {
        let data_points = data.len();
        
        if data_points == 0 {
            return MetricInsights {
                current: 0.0,
                min: 0.0,
                max: 0.0,
                data_points: 0,
            };
        }
        
        let current = data.back().unwrap().value;
        let (min, max) = if data_points == 1 {
            (current, current)
        } else {
            data.iter().map(|tv| tv.value).fold(
                (f64::INFINITY, f64::NEG_INFINITY),
                |(min_acc, max_acc), val| (min_acc.min(val), max_acc.max(val))
            )
        };
        
        MetricInsights {
            current,
            min,
            max,
            data_points,
        }
    }
    
    fn calculate_stats(data: &VecDeque<TimestampedValue>) -> MetricStats {
        if data.is_empty() {
            return MetricStats::default();
        }
        
        let values: Vec<f64> = data.iter().map(|tv| tv.value).collect();
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
        
    fn push_value_to_deque(deque: &mut VecDeque<TimestampedValue>, value: f64, timestamp: u64, max_size: usize) {
        deque.push_back(TimestampedValue { timestamp, value });
        while deque.len() > max_size {
            deque.pop_front();
        }
    }
    
    fn trim_deque(deque: &mut VecDeque<TimestampedValue>, cutoff: u64) {
        while deque.front().map_or(false, |v| v.timestamp < cutoff) {
            deque.pop_front();
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsHistory {
    pub tps: VecDeque<TimestampedValue>,
    pub prompt_tps: VecDeque<TimestampedValue>,
    pub memory_mb: VecDeque<TimestampedValue>,
    pub kv_cache_percent: VecDeque<TimestampedValue>,
    pub kv_cache_tokens: VecDeque<TimestampedValue>,
    
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
            prompt_tps: VecDeque::with_capacity(capacity),
            memory_mb: VecDeque::with_capacity(capacity),
            kv_cache_percent: VecDeque::with_capacity(capacity),
            kv_cache_tokens: VecDeque::with_capacity(capacity),
            max_size: capacity,
        }
    }
    
    pub fn push(&mut self, metrics: &Metrics) {
        let timestamp = current_timestamp();
        
        DataAnalyzer::push_value_to_deque(&mut self.tps, metrics.predicted_tokens_per_sec, timestamp, self.max_size);
        DataAnalyzer::push_value_to_deque(&mut self.prompt_tps, metrics.prompt_tokens_per_sec, timestamp, self.max_size);
        DataAnalyzer::push_value_to_deque(&mut self.memory_mb, metrics.memory_mb, timestamp, self.max_size);
        DataAnalyzer::push_value_to_deque(&mut self.kv_cache_percent, metrics.kv_cache_percent(), timestamp, self.max_size);
        DataAnalyzer::push_value_to_deque(&mut self.kv_cache_tokens, metrics.kv_cache_tokens as f64, timestamp, self.max_size);
        
        self.trim_old_data();
    }
    
    pub fn trim_old_data(&mut self) {
        let cutoff = current_timestamp().saturating_sub(305); // 5 minutes
        
        DataAnalyzer::trim_deque(&mut self.tps, cutoff);
        DataAnalyzer::trim_deque(&mut self.prompt_tps, cutoff);
        DataAnalyzer::trim_deque(&mut self.memory_mb, cutoff);
        DataAnalyzer::trim_deque(&mut self.kv_cache_percent, cutoff);
        DataAnalyzer::trim_deque(&mut self.kv_cache_tokens, cutoff);
    }
    
    pub fn get_values(&self, deque: &VecDeque<TimestampedValue>) -> VecDeque<f64> {
        deque.iter().map(|tv| tv.value).collect()
    }
    
    pub fn clear(&mut self) {
        self.tps.clear();
        self.prompt_tps.clear();
        self.memory_mb.clear();
        self.kv_cache_percent.clear();
        self.kv_cache_tokens.clear();
    }
    
    pub fn get_insights(&self, deque: &VecDeque<TimestampedValue>) -> MetricInsights {
        DataAnalyzer::get_insights(deque)
    }
    
    pub fn calculate_stats(&self, deque: &VecDeque<TimestampedValue>) -> MetricStats {
        DataAnalyzer::calculate_stats(deque)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetricsHistory {
    pub model_name: String,
    pub history: MetricsHistory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllMetricsHistory {
    pub models: std::collections::HashMap<String, MetricsHistory>,
    pub total_llama_memory_mb: VecDeque<TimestampedValue>,
    pub cpu_usage_percent: VecDeque<TimestampedValue>,
    pub memory_usage_percent: VecDeque<TimestampedValue>,
    pub used_memory_gb: VecDeque<TimestampedValue>,
    
    #[serde(skip)]
    pub max_size: usize,
}

impl Default for AllMetricsHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl AllMetricsHistory {
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
            max_size: capacity,
        }
    }
    
    pub fn push(&mut self, all_metrics: &AllMetrics) {
        let timestamp = current_timestamp();
        
        DataAnalyzer::push_value_to_deque(&mut self.total_llama_memory_mb, all_metrics.total_llama_memory_mb, timestamp, self.max_size);
        
        let sys = &all_metrics.system_metrics;
        DataAnalyzer::push_value_to_deque(&mut self.cpu_usage_percent, sys.cpu_usage_percent, timestamp, self.max_size);
        DataAnalyzer::push_value_to_deque(&mut self.memory_usage_percent, sys.memory_usage_percent, timestamp, self.max_size);
        DataAnalyzer::push_value_to_deque(&mut self.used_memory_gb, sys.used_memory_gb, timestamp, self.max_size);
        
        for model_metrics in &all_metrics.models {
            let history = self.models.entry(model_metrics.model_name.clone())
                .or_insert_with(|| MetricsHistory::with_capacity(self.max_size));
            history.push(&model_metrics.metrics);
        }
        
        self.trim_old_data();
    }
    
    pub fn trim_old_data(&mut self) {
        let cutoff = current_timestamp().saturating_sub(300); // 5 minutes
        
        DataAnalyzer::trim_deque(&mut self.total_llama_memory_mb, cutoff);
        DataAnalyzer::trim_deque(&mut self.cpu_usage_percent, cutoff);
        DataAnalyzer::trim_deque(&mut self.memory_usage_percent, cutoff);
        DataAnalyzer::trim_deque(&mut self.used_memory_gb, cutoff);
        
        for (_, history) in self.models.iter_mut() {
            history.trim_old_data();
        }
        
        self.models.retain(|_, history| !history.tps.is_empty());
    }
    
    pub fn clear(&mut self) {
        self.models.clear();
        self.total_llama_memory_mb.clear();
        self.cpu_usage_percent.clear();
        self.memory_usage_percent.clear();
        self.used_memory_gb.clear();
    }
    
    pub fn get_model_history(&self, model_name: &str) -> Option<&MetricsHistory> {
        self.models.get(model_name)
    }
    
    pub fn get_model_names(&self) -> Vec<String> {
        self.models.keys().cloned().collect()
    }
    
    // Unified insights methods using DataAnalyzer
    pub fn get_cpu_insights(&self) -> MetricInsights {
        DataAnalyzer::get_insights(&self.cpu_usage_percent)
    }
    
    pub fn get_system_memory_insights(&self) -> MetricInsights {
        DataAnalyzer::get_insights(&self.memory_usage_percent)
    }
    
    pub fn get_memory_insights(&self) -> MetricInsights {
        DataAnalyzer::get_insights(&self.total_llama_memory_mb)
    }
    
    pub fn calculate_memory_stats(&self) -> MetricStats {
        DataAnalyzer::calculate_stats(&self.total_llama_memory_mb)
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}