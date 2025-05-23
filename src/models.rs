use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

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
    pub tps: f64,
    pub memory_mb: f64,
    pub cache_hit_rate: f64,
}

impl From<MetricsResponse> for Metrics {
    fn from(resp: MetricsResponse) -> Self {
        Self {
            tps: resp.model_count as f64, // Use model count as a proxy for activity
            memory_mb: resp.total_memory_bytes as f64 / 1_048_576.0, // Convert to MB
            cache_hit_rate: 0.0, // Not available from llama-swap
        }
    }
}

#[derive(Debug, Default)]
pub struct MetricsHistory {
    pub tps: VecDeque<f64>,
    pub memory_mb: VecDeque<f64>,
    pub cache_hit_rate: VecDeque<f64>,
}

impl MetricsHistory {
    pub fn new() -> Self {
        Self {
            tps: VecDeque::with_capacity(crate::constants::HISTORY_SIZE),
            memory_mb: VecDeque::with_capacity(crate::constants::HISTORY_SIZE),
            cache_hit_rate: VecDeque::with_capacity(crate::constants::HISTORY_SIZE),
        }
    }

    pub fn push(&mut self, metrics: &Metrics) {
        // Add new values
        self.tps.push_back(metrics.tps);
        self.memory_mb.push_back(metrics.memory_mb);
        self.cache_hit_rate.push_back(metrics.cache_hit_rate);

        // Remove old values if over capacity
        if self.tps.len() > crate::constants::HISTORY_SIZE {
            self.tps.pop_front();
        }
        if self.memory_mb.len() > crate::constants::HISTORY_SIZE {
            self.memory_mb.pop_front();
        }
        if self.cache_hit_rate.len() > crate::constants::HISTORY_SIZE {
            self.cache_hit_rate.pop_front();
        }
    }

    pub fn clear(&mut self) {
        self.tps.clear();
        self.memory_mb.clear();
        self.cache_hit_rate.clear();
    }
}