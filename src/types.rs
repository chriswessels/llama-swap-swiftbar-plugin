use crate::models::{AllMetrics, AllMetricsHistory};
use crate::state_model::{AgentState, DisplayState, ModelState, PollingMode};
use reqwest::blocking::Client;
use std::collections::HashMap;
use std::error::Error;
use std::time::{Duration, Instant};

/// Detailed service status tracking different layers of service management
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ServiceStatus {
    pub plist_installed: bool,
    pub launchctl_loaded: bool,
    pub process_running: bool,
    pub api_responsive: bool,
}

impl ServiceStatus {
    pub fn new() -> Self {
        Self {
            plist_installed: false,
            launchctl_loaded: false,
            process_running: false,
            api_responsive: false,
        }
    }

    pub fn update(&mut self, api_success: bool) {
        self.plist_installed = crate::commands::is_service_installed().unwrap_or(false);
        self.launchctl_loaded = crate::service::is_service_loaded();
        self.process_running = crate::service::is_service_running();
        self.api_responsive = api_success;
    }

    /// Service is fully operational (all layers working)
    pub fn is_fully_running(&self) -> bool {
        self.plist_installed && self.launchctl_loaded && self.process_running && self.api_responsive
    }

    /// Get user-friendly status description
    pub fn status_description(&self) -> &'static str {
        match (
            self.plist_installed,
            self.launchctl_loaded,
            self.process_running,
            self.api_responsive,
        ) {
            (true, true, true, true) => "Running",
            (true, true, true, false) => "Process running but API unresponsive",
            (true, true, false, false) => "Loaded but not running",
            (true, false, false, false) => "Stopped",
            (false, _, _, _) => "Not installed",
            _ => "Unknown state",
        }
    }
}

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
    pub service_status: ServiceStatus,

    // Timing for state transitions
    last_state_change: Instant,
}

impl PluginState {
    pub fn new() -> Result<Self> {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(crate::constants::API_TIMEOUT_SECS))
            .build()?;

        // Initialize service status
        let mut service_status = ServiceStatus::new();
        service_status.update(false); // API not tested yet

        // Determine initial agent state
        let binary_available = crate::commands::find_llama_swap_binary().is_ok();
        let agent_state = AgentState::from_system_check(
            service_status.plist_installed,
            binary_available,
            service_status.is_fully_running(),
        );

        Ok(Self {
            http_client,
            metrics_history: AllMetricsHistory::new(),
            current_all_metrics: None,
            error_count: 0,
            agent_state,
            polling_mode: PollingMode::Idle,
            model_states: HashMap::new(),
            service_status,
            last_state_change: Instant::now(),
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
            eprintln!(
                "Polling mode: {} -> {} ({})",
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
            let (total_processing, total_deferred) =
                all_metrics
                    .models
                    .iter()
                    .fold((0, 0), |(proc, def), model| {
                        (
                            proc + model.metrics.requests_processing,
                            def + model.metrics.requests_deferred,
                        )
                    });

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
        // Always collect system metrics regardless of API state
        let mut system = sysinfo::System::new_all();
        let system_metrics = crate::metrics::collect_system_metrics(&mut system);
        let llama_memory_mb = crate::metrics::get_llama_server_memory_mb(&system);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Store system metrics independently using CircularQueue direct push
        self.metrics_history
            .cpu_usage_percent
            .push(crate::models::TimestampedValue {
                timestamp,
                value: system_metrics.cpu_usage_percent,
            });
        self.metrics_history
            .memory_usage_percent
            .push(crate::models::TimestampedValue {
                timestamp,
                value: system_metrics.memory_usage_percent,
            });
        self.metrics_history
            .used_memory_gb
            .push(crate::models::TimestampedValue {
                timestamp,
                value: system_metrics.used_memory_gb,
            });
        self.metrics_history
            .total_llama_memory_mb
            .push(crate::models::TimestampedValue {
                timestamp,
                value: llama_memory_mb,
            });

        // Check API connectivity first, then update agent state based on that
        let api_success = match crate::metrics::fetch_all_metrics(&self.http_client) {
            Ok(all_metrics) => {
                self.handle_metrics_success(all_metrics);
                true
            }
            Err(e) => {
                eprintln!(
                    "Debug: Metrics fetch failed in state {:?}: {}",
                    self.agent_state, e
                );
                self.handle_metrics_error(e);
                false
            }
        };

        // Update service status with API connectivity result
        self.service_status.update(api_success);

        // Update agent state with proper transitions, using comprehensive service status
        self.update_agent_state();

        self.update_polling_mode();
    }

    pub fn update_agent_state(&mut self) {
        let old_state = self.agent_state;

        // Get current system status
        let binary_available = crate::commands::find_llama_swap_binary().is_ok();

        // Compute new state using comprehensive service status
        let new_state = AgentState::from_system_check(
            self.service_status.plist_installed,
            binary_available,
            self.service_status.is_fully_running(),
        );

        if matches!(old_state, AgentState::Stopped) && matches!(new_state, AgentState::Running) {
            // Direct transition to Running
            self.agent_state = AgentState::Running;
        } else {
            self.agent_state = new_state;
        }

        if self.agent_state != old_state {
            self.last_state_change = Instant::now();
            eprintln!("Agent state: {old_state:?} -> {:?}", self.agent_state);
        }
    }

    pub fn handle_metrics_success(&mut self, all_metrics: AllMetrics) {
        // Don't call push() which would overwrite independently collected system metrics
        // Instead, only update model-specific metrics and current state

        // Update model histories
        for model_metrics in &all_metrics.models {
            let history = self
                .metrics_history
                .models
                .entry(model_metrics.model_name.clone())
                .or_default();
            history.push(&model_metrics.metrics);
        }

        // Don't update llama memory here - it's collected independently in update_state
        // to avoid overwriting good data with API response zeros when models are unloaded

        // Trim old data for all metrics
        self.metrics_history.trim_old_data();

        self.current_all_metrics = Some(all_metrics.clone());
        self.error_count = 0;

        // Update model states
        self.update_model_states(&all_metrics);
    }

    pub fn handle_metrics_error(&mut self, error: Box<dyn Error>) {
        eprintln!("Metrics fetch failed: {error}");
        self.error_count += 1;

        // Clear current model states since we can't verify their current status
        self.model_states.clear();

        // Clear current metrics snapshot, but preserve all historical data
        // All metrics (system, model, llama memory) are preserved within the 5-minute retention window
        // Natural time-based cleanup will handle old data automatically
        self.current_all_metrics = None;

        // Note: All historical metrics (system, model, llama memory) are preserved across API failures
        // and service issues, only cleaned up by the natural 5-minute retention window
    }

    pub fn update_model_states(&mut self, all_metrics: &AllMetrics) {
        // Remove models that no longer exist
        let current_model_names: std::collections::HashSet<String> = all_metrics
            .models
            .iter()
            .map(|m| m.model_name.clone())
            .collect();
        self.model_states
            .retain(|name, _| current_model_names.contains(name));

        // Update or create states for each model
        for model_data in &all_metrics.models {
            let state = match model_data.model_state {
                crate::models::ModelState::Loading => ModelState::Loading,
                crate::models::ModelState::Running => ModelState::Running,
                crate::models::ModelState::Unknown => ModelState::Unknown,
            };
            self.model_states
                .insert(model_data.model_name.clone(), state);
        }
    }

    pub fn get_display_state(&self) -> DisplayState {
        match self.agent_state {
            AgentState::NotReady { .. } => DisplayState::AgentNotLoaded,

            AgentState::Stopped => DisplayState::ServiceStopped, // Fix: Ready to start
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
