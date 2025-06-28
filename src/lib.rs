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
pub mod types;

// Re-export error type is now in types module

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

// Re-export main types
pub use crate::types::{PluginState, Result};