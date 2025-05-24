// Module declarations
pub mod charts;
pub mod commands;
pub mod constants;
pub mod icons;
pub mod menu;
pub mod metrics;
pub mod models;
pub mod service;

// Re-export error type
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

// Re-export commonly used types
pub use crate::models::{
    Metrics,
    MetricStats,
    MetricsHistory,
    ServiceStatus,
    TimestampedValue,
};