use std::time::Duration;

/// Reason why the agent is not ready
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NotReadyReason {
    BinaryNotFound,
    PlistMissing,
}


/// Simplified agent states
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AgentState {
    NotReady { reason: NotReadyReason },
    Stopped,
    Starting,
    Running,
}

impl AgentState {
    pub fn from_system_check(plist_installed: bool, binary_available: bool, service_running: bool) -> Self {
        match (plist_installed, binary_available, service_running) {
            (_, _, true) => AgentState::Running,
            (true, true, false) => AgentState::Stopped,  // Ready to start
            (false, false, _) => AgentState::NotReady { reason: NotReadyReason::BinaryNotFound },
            (false, true, _) => AgentState::NotReady { reason: NotReadyReason::PlistMissing },
            (true, false, _) => AgentState::NotReady { reason: NotReadyReason::BinaryNotFound }, // Fix: plist exists but binary missing
        }
    }
    
}

/// Display state computed from agent and model states
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DisplayState {
    AgentNotLoaded,
    AgentStarting,
    ServiceStopped,        // Service stopped but ready to start
    ServiceLoadedNoModel,  // Service running but no models
    ModelLoading,
    ModelProcessingQueue,
    ModelReady,
}

impl DisplayState {
    pub fn status_message(&self) -> &'static str {
        match self {
            DisplayState::AgentNotLoaded => "Missing requirements",
            DisplayState::AgentStarting => "Starting agent...",
            DisplayState::ServiceStopped => "Service stopped",
            DisplayState::ServiceLoadedNoModel => "No models loaded",
            DisplayState::ModelLoading => "Loading model...",
            DisplayState::ModelProcessingQueue => "Processing queue...",
            DisplayState::ModelReady => "Model ready",
        }
    }

    pub fn icon_color(&self) -> &'static str {
        match self {
            DisplayState::AgentNotLoaded => "red",        // Problems - missing requirements
            DisplayState::ServiceStopped => "red",        // Problems - service needs to be started
            DisplayState::ServiceLoadedNoModel => "grey", // Idle - service running but no models
            DisplayState::AgentStarting => "yellow",      // Transitional - starting up
            DisplayState::ModelLoading => "yellow",       // Transitional - loading model
            DisplayState::ModelReady => "green",          // Ready - models loaded and idle
            DisplayState::ModelProcessingQueue => "blue", // Active - processing requests
        }
    }
}

/// Simplified polling mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PollingMode {
    Idle,    // 3s - no activity
    Active,  // 1s - active processing
    Starting, // 2s - transitioning
}

impl PollingMode {
    pub fn interval(&self) -> Duration {
        match self {
            PollingMode::Idle => Duration::from_secs(3),
            PollingMode::Active => Duration::from_secs(1),
            PollingMode::Starting => Duration::from_secs(2),
        }
    }
    
    pub fn description(&self) -> &'static str {
        match self {
            PollingMode::Idle => "Idle",
            PollingMode::Active => "Active", 
            PollingMode::Starting => "Starting",
        }
    }
    
    /// Determine polling mode based on state changes and activity
    pub fn compute(
        _current: PollingMode,
        state_changed: bool,
        has_activity: bool,
        last_change_elapsed: Duration,
    ) -> PollingMode {
        const STATE_CHANGE_DURATION: Duration = Duration::from_secs(5);
        
        match (state_changed, has_activity, last_change_elapsed < STATE_CHANGE_DURATION) {
            (true, _, _) => PollingMode::Starting,  // Just changed
            (_, _, true) => PollingMode::Starting,   // Recently changed
            (_, true, _) => PollingMode::Active,     // Has activity
            _ => PollingMode::Idle,                  // No activity
        }
    }
}

/// Simple model state (no more duplication with state machine)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModelState {
    Unknown,
    Loading,
    Running,
}

impl ModelState {
    pub fn is_loading(&self) -> bool {
        matches!(self, ModelState::Loading)
    }
}