use circular_queue::CircularQueue;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModelState {
    Running,
    Loading,
    Unknown,
}

#[derive(Debug, Deserialize)]
pub struct RunningModel {
    pub model: String,
    pub state: String,
}

impl RunningModel {
    pub fn model_state(&self) -> ModelState {
        match self.state.as_str() {
            "ready" => ModelState::Running,
            "starting" | "stopping" => ModelState::Loading,
            _ => ModelState::Unknown,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RunningResponse {
    pub running: Vec<RunningModel>,
}

#[derive(Debug, Clone)]
pub struct ModelMetrics {
    pub model_name: String,
    pub model_state: ModelState,
    pub metrics: Metrics,
}

#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub cpu_usage_percent: f64,
    pub used_memory_gb: f64,
    pub memory_usage_percent: f64,
}

#[derive(Debug, Clone)]
pub struct AllMetrics {
    pub models: Vec<ModelMetrics>,
    #[allow(dead_code)]
    pub total_llama_memory_mb: f64,
    #[allow(dead_code)]
    pub system_metrics: SystemMetrics,
}

#[derive(Debug, Clone, Default)]
pub struct Metrics {
    pub prompt_tokens_per_sec: f64,
    pub predicted_tokens_per_sec: f64,
    pub requests_processing: u32,
    pub requests_deferred: u32,
    pub n_decode_total: u32,
    pub memory_mb: f64,
}

impl Metrics {
    pub fn queue_status(&self) -> String {
        match (self.requests_processing, self.requests_deferred) {
            (0, 0) => "Idle".to_string(),
            (n, 0) => format!("{n} active"),
            (0, n) => format!("{n} queued"),
            (p, q) => format!("{p} active, {q} queued"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimestampedValue {
    pub timestamp: u64,
    pub value: f64,
}

// MetricInsights has been merged into MetricStats

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MetricStats {
    pub mean: f64,
    pub min: f64,
    pub max: f64,
    pub std_dev: f64,
    pub count: usize,
    pub current: f64,
}

impl MetricStats {
    pub fn time_context(&self, oldest_timestamp: u64, newest_timestamp: u64) -> String {
        match self.count {
            0 => String::new(),
            1 => "(now)".to_string(),
            _ => {
                let duration_secs = newest_timestamp.saturating_sub(oldest_timestamp);
                let time_text = match duration_secs {
                    s if s < 60 => format!("{s}s"),
                    s if s < 3600 => {
                        let minutes = s / 60;
                        let seconds = s % 60;
                        if seconds == 0 {
                            format!("{minutes}m")
                        } else {
                            format!("{minutes}m {seconds}s")
                        }
                    }
                    s => {
                        let hours = s / 3600;
                        let remaining_minutes = (s % 3600) / 60;
                        if remaining_minutes == 0 {
                            format!("{hours}h")
                        } else {
                            format!("{hours}h {remaining_minutes}m")
                        }
                    }
                };
                format!("{} samples over {}", self.count, time_text)
            }
        }
    }
}

// Unified analysis operations
pub struct DataAnalyzer;

impl DataAnalyzer {
    #[allow(dead_code)]
    pub fn get_stats(data: &VecDeque<TimestampedValue>) -> MetricStats {
        if data.is_empty() {
            return MetricStats::default();
        }

        let values: Vec<f64> = data.iter().map(|tv| tv.value).collect();
        let sum: f64 = values.iter().sum();
        let count = values.len() as f64;
        let mean = sum / count;
        let current = data.back().unwrap().value;

        let min = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

        let variance = values.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / count;
        let std_dev = variance.sqrt();

        MetricStats {
            mean,
            min,
            max,
            std_dev,
            count: values.len(),
            current,
        }
    }

    #[allow(dead_code)]
    pub fn push_value_to_deque(
        deque: &mut VecDeque<TimestampedValue>,
        value: f64,
        timestamp: u64,
        max_size: usize,
    ) {
        deque.push_back(TimestampedValue { timestamp, value });
        while deque.len() > max_size {
            deque.pop_front();
        }
    }

    #[allow(dead_code)]
    pub fn trim_deque(deque: &mut VecDeque<TimestampedValue>, cutoff: u64) {
        while deque.front().is_some_and(|v| v.timestamp < cutoff) {
            deque.pop_front();
        }
    }

    pub fn get_stats_from_circular_queue(cq: &CircularQueue<TimestampedValue>) -> MetricStats {
        if cq.is_empty() {
            return MetricStats::default();
        }

        // Use iter().rev() to get oldest-to-newest order (like VecDeque)
        let values: Vec<f64> = cq.iter().rev().map(|tv| tv.value).collect();
        let sum: f64 = values.iter().sum();
        let count = values.len() as f64;
        let mean = sum / count;
        let current = cq.iter().next().unwrap().value; // First in CircularQueue is newest

        let min = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

        let variance = values.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / count;
        let std_dev = variance.sqrt();

        MetricStats {
            mean,
            min,
            max,
            std_dev,
            count: values.len(),
            current,
        }
    }

    pub fn trim_circular_queue(cq: &mut CircularQueue<TimestampedValue>, cutoff: u64) {
        // For CircularQueue, we need to rebuild with only valid entries
        // Use rev() to get oldest-to-newest order, then push in that order
        // so that newest ends up being the first in CircularQueue iteration
        let valid_entries: Vec<TimestampedValue> = cq
            .iter()
            .rev() // Get oldest-to-newest order
            .filter(|v| v.timestamp >= cutoff)
            .cloned()
            .collect();

        // Clear and rebuild (pushing oldest first, newest last)
        cq.clear();
        for entry in valid_entries {
            cq.push(entry);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsHistory {
    pub tps: CircularQueue<TimestampedValue>,
    pub prompt_tps: CircularQueue<TimestampedValue>,
    pub memory_mb: CircularQueue<TimestampedValue>,
    pub queue_size: CircularQueue<TimestampedValue>,

    #[serde(skip)]
    #[allow(dead_code)]
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
            tps: CircularQueue::with_capacity(capacity),
            prompt_tps: CircularQueue::with_capacity(capacity),
            memory_mb: CircularQueue::with_capacity(capacity),
            queue_size: CircularQueue::with_capacity(capacity),
            max_size: capacity,
        }
    }

    pub fn push(&mut self, metrics: &Metrics) {
        let timestamp = current_timestamp();

        // CircularQueue automatically handles capacity, no manual size management needed
        self.tps.push(TimestampedValue {
            timestamp,
            value: metrics.predicted_tokens_per_sec,
        });
        self.prompt_tps.push(TimestampedValue {
            timestamp,
            value: metrics.prompt_tokens_per_sec,
        });
        self.memory_mb.push(TimestampedValue {
            timestamp,
            value: metrics.memory_mb,
        });
        self.queue_size.push(TimestampedValue {
            timestamp,
            value: (metrics.requests_processing + metrics.requests_deferred) as f64,
        });

        self.trim_old_data();
    }

    pub fn trim_old_data(&mut self) {
        let cutoff = current_timestamp().saturating_sub(305); // 5 minutes

        DataAnalyzer::trim_circular_queue(&mut self.tps, cutoff);
        DataAnalyzer::trim_circular_queue(&mut self.prompt_tps, cutoff);
        DataAnalyzer::trim_circular_queue(&mut self.memory_mb, cutoff);
        DataAnalyzer::trim_circular_queue(&mut self.queue_size, cutoff);
    }

    pub fn get_stats(&self, circular_queue: &CircularQueue<TimestampedValue>) -> MetricStats {
        DataAnalyzer::get_stats_from_circular_queue(circular_queue)
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
    pub total_llama_memory_mb: CircularQueue<TimestampedValue>,
    pub cpu_usage_percent: CircularQueue<TimestampedValue>,
    pub memory_usage_percent: CircularQueue<TimestampedValue>,
    pub used_memory_gb: CircularQueue<TimestampedValue>,

    #[serde(skip)]
    #[allow(dead_code)]
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
            total_llama_memory_mb: CircularQueue::with_capacity(capacity),
            cpu_usage_percent: CircularQueue::with_capacity(capacity),
            memory_usage_percent: CircularQueue::with_capacity(capacity),
            used_memory_gb: CircularQueue::with_capacity(capacity),
            max_size: capacity,
        }
    }

    #[allow(dead_code)]
    pub fn push(&mut self, all_metrics: &AllMetrics) {
        let timestamp = current_timestamp();

        // CircularQueue automatically handles capacity
        self.total_llama_memory_mb.push(TimestampedValue {
            timestamp,
            value: all_metrics.total_llama_memory_mb,
        });

        let sys = &all_metrics.system_metrics;
        self.cpu_usage_percent.push(TimestampedValue {
            timestamp,
            value: sys.cpu_usage_percent,
        });
        self.memory_usage_percent.push(TimestampedValue {
            timestamp,
            value: sys.memory_usage_percent,
        });
        self.used_memory_gb.push(TimestampedValue {
            timestamp,
            value: sys.used_memory_gb,
        });

        for model_metrics in &all_metrics.models {
            let history = self
                .models
                .entry(model_metrics.model_name.clone())
                .or_insert_with(|| MetricsHistory::with_capacity(self.max_size));
            history.push(&model_metrics.metrics);
        }

        self.trim_old_data();
    }

    pub fn trim_old_data(&mut self) {
        let cutoff = current_timestamp().saturating_sub(300); // 5 minutes

        DataAnalyzer::trim_circular_queue(&mut self.total_llama_memory_mb, cutoff);
        DataAnalyzer::trim_circular_queue(&mut self.cpu_usage_percent, cutoff);
        DataAnalyzer::trim_circular_queue(&mut self.memory_usage_percent, cutoff);
        DataAnalyzer::trim_circular_queue(&mut self.used_memory_gb, cutoff);

        for (_, history) in self.models.iter_mut() {
            history.trim_old_data();
        }

        // Only remove model histories if they have no data at all (never had any metrics)
        // This preserves historical data for unloaded models for the full 5-minute window
        self.models.retain(|_, history| {
            !history.tps.is_empty()
                || !history.prompt_tps.is_empty()
                || !history.memory_mb.is_empty()
                || !history.queue_size.is_empty()
        });
    }

    pub fn get_model_history(&self, model_name: &str) -> Option<&MetricsHistory> {
        self.models.get(model_name)
    }

    // Unified stats methods using DataAnalyzer
    pub fn get_cpu_stats(&self) -> MetricStats {
        DataAnalyzer::get_stats_from_circular_queue(&self.cpu_usage_percent)
    }

    pub fn get_system_memory_stats(&self) -> MetricStats {
        DataAnalyzer::get_stats_from_circular_queue(&self.memory_usage_percent)
    }

    pub fn get_memory_stats(&self) -> MetricStats {
        DataAnalyzer::get_stats_from_circular_queue(&self.total_llama_memory_mb)
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
