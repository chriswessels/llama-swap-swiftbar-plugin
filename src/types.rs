use crate::models::{AllMetricsHistory, AllMetrics};
use crate::state_model::{AgentState, DisplayState, PollingMode, ModelState};
use crate::metrics;
use reqwest::blocking::Client;
use std::collections::HashMap;
use std::error::Error;
use std::time::{Duration, Instant};

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// Error helper functions to reduce boilerplate
pub mod error_helpers {
    use super::Result;
    
    /// Convert any error to our Result type with a context message
    pub fn with_context<T, E: std::fmt::Display>(
        result: std::result::Result<T, E>,
        context: &str,
    ) -> Result<T> {
        result.map_err(|e| format!("{context}: {e}").into())
    }
    
    /// Get HOME directory or return error
    pub fn get_home_dir() -> Result<String> {
        std::env::var("HOME").map_err(|_| "Failed to get HOME directory".into())
    }
    
    /// Common error contexts
    pub const CONNECT_API: &str = "Failed to connect to API";
    pub const PARSE_JSON: &str = "Failed to parse JSON";
    pub const START_SERVICE: &str = "Failed to start service";
    pub const STOP_SERVICE: &str = "Failed to stop service";
    pub const GET_USER_ID: &str = "Failed to get user ID";
    pub const CREATE_DIR: &str = "Failed to create directory";
    pub const CREATE_FILE: &str = "Failed to create file";
    pub const EXEC_COMMAND: &str = "Failed to execute command";
}

pub struct PluginState {
    pub http_client: Client,
    pub metrics_history: AllMetricsHistory,
    pub current_all_metrics: Option<AllMetrics>,
    pub error_count: usize,
    
    // Simplified state
    pub agent_state: AgentState,
    pub polling_mode: PollingMode,
    pub model_states: HashMap<String, ModelState>,
    
    // Timing for state transitions
    last_state_change: Instant,
    startup_time: Option<Instant>,
}

impl PluginState {
    pub fn new() -> Result<Self> {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(crate::constants::API_TIMEOUT_SECS))
            .build()?;

        // Determine initial agent state
        let plist_installed = crate::commands::is_service_installed().unwrap_or(false);
        let binary_available = crate::commands::find_llama_swap_binary().is_ok();
        let service_running = crate::service::is_service_running();
        let agent_state = AgentState::from_system_check(plist_installed, binary_available, service_running);

        Ok(Self {
            http_client,
            metrics_history: AllMetricsHistory::new(),
            current_all_metrics: None,
            error_count: 0,
            agent_state,
            polling_mode: PollingMode::Idle,
            model_states: HashMap::new(),
            last_state_change: Instant::now(),
            startup_time: None,
        })
    }
    
    pub fn update_polling_mode(&mut self) {
        let old_mode = self.polling_mode;
        let state_changed = self.last_state_change.elapsed() < Duration::from_millis(100);
        let has_activity = self.has_queue_activity();
        
        self.polling_mode = PollingMode::compute(
            self.polling_mode,
            state_changed,
            has_activity,
            self.last_state_change.elapsed(),
        );
        
        if self.polling_mode != old_mode {
            eprintln!("Polling mode: {} -> {} ({})", 
                old_mode.description(),
                self.polling_mode.description(),
                self.get_mode_reason()
            );
        }
    }
    
    pub fn has_queue_activity(&self) -> bool {
        self.current_all_metrics
            .as_ref()
            .is_some_and(|all_metrics| {
                all_metrics.models.iter().any(|model| {
                    model.metrics.requests_processing > 0 || model.metrics.requests_deferred > 0
                })
            })
    }
    
    pub fn get_mode_reason(&self) -> String {
        if let Some(ref all_metrics) = self.current_all_metrics {
            let (total_processing, total_deferred) = all_metrics.models.iter().fold(
                (0, 0),
                |(proc, def), model| {
                    (proc + model.metrics.requests_processing, def + model.metrics.requests_deferred)
                }
            );
            
            match (total_processing, total_deferred) {
                (p, _) if p > 0 => format!("processing {p} requests"),
                (_, d) if d > 0 => format!("{d} requests queued"),
                _ => "no queue activity".to_string(),
            }
        } else {
            "no queue activity".to_string()
        }
    }
    
    pub fn update_state(&mut self) {
        // Update agent state with proper transitions
        self.update_agent_state();
        
        // Determine program state based on agent and model status
        match metrics::fetch_all_metrics(&self.http_client) {
            Ok(all_metrics) => self.handle_metrics_success(all_metrics),
            Err(e) => self.handle_metrics_error(e),
        }
        
        self.update_polling_mode();
    }
    
    pub fn update_agent_state(&mut self) {
        let old_state = self.agent_state;
        
        // Get current system status
        let plist_installed = crate::commands::is_service_installed().unwrap_or(false);
        let binary_available = crate::commands::find_llama_swap_binary().is_ok();
        let service_running = crate::service::is_service_running();
        
        // Compute new state
        let new_state = AgentState::from_system_check(plist_installed, binary_available, service_running);
        
        // Handle Starting -> Running/Stopped transition after timeout
        if let AgentState::Starting = self.agent_state {
            if let Some(startup_time) = self.startup_time {
                if startup_time.elapsed() >= Duration::from_secs(5) {
                    self.agent_state = if service_running {
                        AgentState::Running
                    } else {
                        AgentState::Stopped
                    };
                    self.startup_time = None;
                }
            }
        } else if matches!(old_state, AgentState::Stopped) && matches!(new_state, AgentState::Running) {
            // Transition through Starting state
            self.agent_state = AgentState::Starting;
            self.startup_time = Some(Instant::now());
        } else {
            self.agent_state = new_state;
        }
        
        if self.agent_state != old_state {
            self.last_state_change = Instant::now();
            eprintln!("Agent state: {old_state:?} -> {:?}", self.agent_state);
        }
    }
    
    pub fn handle_metrics_success(&mut self, all_metrics: AllMetrics) {
        self.metrics_history.push(&all_metrics);
        self.current_all_metrics = Some(all_metrics.clone());
        self.error_count = 0;
        
        // Update model states
        self.update_model_states(&all_metrics);
    }
    
    pub fn handle_metrics_error(&mut self, error: Box<dyn Error>) {
        eprintln!("Metrics fetch failed: {error}");
        self.error_count += 1;
        
        // Clear model states on error
        self.model_states.clear();
        
        if !matches!(self.agent_state, AgentState::Running) {
            self.current_all_metrics = None;
            self.metrics_history.models.clear();
            self.metrics_history.total_llama_memory_mb.clear();
            self.metrics_history.cpu_usage_percent.clear();
            self.metrics_history.memory_usage_percent.clear();
            self.metrics_history.used_memory_gb.clear();
        } else {
            self.current_all_metrics = None;
        }
    }
    
    pub fn update_model_states(&mut self, all_metrics: &AllMetrics) {
        // Remove models that no longer exist
        let current_model_names: std::collections::HashSet<String> = all_metrics.models.iter().map(|m| m.model_name.clone()).collect();
        self.model_states.retain(|name, _| current_model_names.contains(name));
        
        // Update or create states for each model
        for model_data in &all_metrics.models {
            let state = match model_data.model_state {
                crate::models::ModelState::Loading => ModelState::Loading,
                crate::models::ModelState::Running => ModelState::Running,
                crate::models::ModelState::Unknown => ModelState::Unknown,
            };
            self.model_states.insert(model_data.model_name.clone(), state);
        }
    }
    
    pub fn get_display_state(&self) -> DisplayState {
        match self.agent_state {
            AgentState::NotReady { .. } => DisplayState::AgentNotLoaded,
            AgentState::Starting => DisplayState::AgentStarting,
            AgentState::Stopped => DisplayState::ServiceStopped,  // Fix: Ready to start
            AgentState::Running => {
                if self.model_states.is_empty() {
                    DisplayState::ServiceLoadedNoModel
                } else if self.has_loading_models() {
                    DisplayState::ModelLoading
                } else if self.has_queue_activity() {
                    DisplayState::ModelProcessingQueue
                } else {
                    DisplayState::ModelReady
                }
            }
        }
    }
    
    pub fn has_loading_models(&self) -> bool {
        self.model_states.values().any(|state| state.is_loading())
    }
}