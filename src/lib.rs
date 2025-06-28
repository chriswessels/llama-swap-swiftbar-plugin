// Module declarations
pub mod charts;
pub mod commands;
pub mod constants;
pub mod icons;
pub mod menu;
pub mod metrics;
pub mod models;
pub mod service;
pub mod state_machines;

// Re-export error type
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

// Re-export commonly used types
pub use crate::models::{
    Metrics,
    MetricStats,
    MetricsHistory,
    ProgramState,
    AgentState,
    TimestampedValue,
};

// Re-export state machine types
pub use crate::state_machines::{
    agent::{AgentStates, AgentStateMachine},
    program::{ProgramStates, ProgramStateMachine},
    polling_mode::{PollingModeStates, PollingModeStateMachine},
    model::{ModelStates, ModelStateMachine},
};