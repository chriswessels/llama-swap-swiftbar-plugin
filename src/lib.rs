// Re-export main types and modules for external use
pub use crate::models::{ServiceStatus, MetricsHistory, TimestampedValue, MetricStats};

// Module declarations
pub mod models;
pub mod menu;
pub mod constants;
pub mod icons;
pub mod charts;
pub mod metrics;
pub mod commands;
pub mod service;

// Re-export error type
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

// Define PluginState for library use
#[derive(Debug)]
pub struct PluginState {
    pub current_status: ServiceStatus,
    pub metrics_history: MetricsHistory,
    pub error_count: u32,
}