use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ServiceStatus {
    Running,
    Stopped,
    Unknown,
}

#[derive(Debug, Deserialize)]
pub struct MetricsResponse {
    pub tps: f64,
    #[serde(rename = "memory_bytes")]
    pub memory_bytes: u64,
    #[serde(rename = "cache_hits")]
    pub cache_hits: u64,
    #[serde(rename = "cache_misses")]
    pub cache_misses: u64,
    // Additional fields can be added as needed
}

#[derive(Debug)]
pub struct Metrics {
    pub tps: f64,
    pub memory_mb: f64,
    pub cache_hit_rate: f64,
}

impl From<MetricsResponse> for Metrics {
    fn from(resp: MetricsResponse) -> Self {
        let total_cache = resp.cache_hits + resp.cache_misses;
        let cache_hit_rate = if total_cache > 0 {
            (resp.cache_hits as f64 / total_cache as f64) * 100.0
        } else {
            0.0
        };

        Self {
            tps: resp.tps,
            memory_mb: resp.memory_bytes as f64 / 1_048_576.0, // Convert to MB
            cache_hit_rate,
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