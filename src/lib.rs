// Module declarations
pub mod charts;
pub mod commands;
pub mod constants;
pub mod icons;
pub mod menu;
pub mod metrics;
pub mod models;
pub mod service;
pub mod state_model;
pub mod types;

// Re-export error type is now in types module

// Re-export commonly used types
pub use crate::models::{MetricStats, Metrics, MetricsHistory, TimestampedValue};

// Re-export simplified state model types
pub use crate::state_model::{AgentState, DisplayState, ModelState, NotReadyReason, PollingMode};

// Re-export main types
pub use crate::types::{PluginState, Result};
