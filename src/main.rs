mod constants;
mod models;
mod menu;
mod metrics;
mod charts;
mod icons;
mod commands;
mod service;

use crate::models::{AllMetricsHistory, AllMetrics, ProgramState, AgentState};
use reqwest::blocking::Client;
use std::error::Error;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PollingMode {
    Idle,
    Active,
    Starting,
}

impl PollingMode {
    pub fn interval_secs(&self) -> u64 {
        match self {
            PollingMode::Idle => constants::UPDATE_INTERVAL_SECS,
            PollingMode::Active => constants::ACTIVE_INTERVAL_SECS,
            PollingMode::Starting => constants::STARTING_INTERVAL_SECS,
        }
    }
    
    pub fn description(&self) -> &'static str {
        match self {
            PollingMode::Idle => "idle polling",
            PollingMode::Active => "active polling",
            PollingMode::Starting => "transition polling",
        }
    }
}

pub struct PluginState {
    pub http_client: Client,
    pub metrics_history: AllMetricsHistory,
    pub current_program_state: ProgramState,
    pub current_agent_state: AgentState,
    pub current_all_metrics: Option<AllMetrics>,
    pub error_count: usize,
    pub polling_mode: PollingMode,
    pub last_program_state: ProgramState,
    pub last_agent_state: AgentState,
    pub mode_change_time: Instant,
    pub agent_state_change_time: Instant,
}

impl PluginState {
    fn new() -> Result<Self> {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(constants::API_TIMEOUT_SECS))
            .build()?;

        Ok(Self {
            http_client,
            metrics_history: AllMetricsHistory::new(),
            current_program_state: ProgramState::AgentNotLoaded,
            current_agent_state: AgentState::NotInstalled,
            current_all_metrics: None,
            error_count: 0,
            polling_mode: PollingMode::Starting,
            last_program_state: ProgramState::AgentNotLoaded,
            last_agent_state: AgentState::NotInstalled,
            mode_change_time: Instant::now(),
            agent_state_change_time: Instant::now(),
        })
    }
    
    fn update_polling_mode(&mut self) {
        let new_mode = self.determine_polling_mode();
        
        if new_mode != self.polling_mode {
            eprintln!("Polling mode: {} -> {} ({})", 
                self.polling_mode.description(),
                new_mode.description(),
                self.get_mode_reason()
            );
            
            self.polling_mode = new_mode;
            self.mode_change_time = Instant::now();
        }
    }
    
    fn determine_polling_mode(&self) -> PollingMode {
        // State transition takes priority
        if self.current_program_state != self.last_program_state {
            return PollingMode::Starting;
        }
        
        // Stay in Starting mode for minimum duration
        if self.polling_mode == PollingMode::Starting && 
           self.mode_change_time.elapsed() < Duration::from_secs(constants::MIN_STARTING_DURATION_SECS) {
            return PollingMode::Starting;
        }
        
        // Check for active processing
        if self.has_queue_activity() {
            return PollingMode::Active;
        }
        
        // Default based on service status
        match self.current_program_state {
            ProgramState::ModelReady | ProgramState::ServiceLoadedNoModel => PollingMode::Idle,
            ProgramState::ModelProcessingQueue => PollingMode::Active,
            _ => PollingMode::Starting,
        }
    }
    
    fn has_queue_activity(&self) -> bool {
        self.current_all_metrics
            .as_ref()
            .map_or(false, |all_metrics| {
                all_metrics.models.iter().any(|model| {
                    model.metrics.requests_processing > 0 || model.metrics.requests_deferred > 0
                })
            })
    }
    
    fn get_mode_reason(&self) -> String {
        if self.current_program_state != self.last_program_state {
            return format!("program state changed: {:?} -> {:?}", self.last_program_state, self.current_program_state);
        }
        
        if let Some(ref all_metrics) = self.current_all_metrics {
            let (total_processing, total_deferred) = all_metrics.models.iter().fold(
                (0, 0),
                |(proc, def), model| {
                    (proc + model.metrics.requests_processing, def + model.metrics.requests_deferred)
                }
            );
            
            match (total_processing, total_deferred) {
                (p, _) if p > 0 => format!("processing {} requests", p),
                (_, d) if d > 0 => format!("{} requests queued", d),
                _ => "no queue activity".to_string(),
            }
        } else {
            "no queue activity".to_string()
        }
    }
    
    fn update_state(&mut self) {
        self.last_program_state = self.current_program_state;
        self.last_agent_state = self.current_agent_state;
        
        // Update agent state with proper transitions
        self.update_agent_state();
        
        // Determine program state based on agent and model status
        match metrics::fetch_all_metrics(&self.http_client) {
            Ok(all_metrics) => self.handle_metrics_success(all_metrics),
            Err(e) => self.handle_metrics_error(e),
        }
        
        self.update_polling_mode();
    }
    
    fn update_agent_state(&mut self) {
        let is_service_running = service::is_service_running(service::DetectionMethod::LaunchctlList);
        
        let new_agent_state = match (self.current_agent_state, is_service_running) {
            // Agent was not installed, now service is running
            (AgentState::NotInstalled, true) => AgentState::Starting,
            
            // Agent was not installed, still not running
            (AgentState::NotInstalled, false) => AgentState::NotInstalled,
            
            // Agent was stopped, now service is running
            (AgentState::Stopped, true) => AgentState::Starting,
            
            // Agent was stopped, still not running
            (AgentState::Stopped, false) => AgentState::Stopped,
            
            // Agent was starting, service still running - check if we should transition to Running
            (AgentState::Starting, true) => {
                // After 5 seconds of starting, consider it running if service is up
                if self.agent_state_change_time.elapsed() > Duration::from_secs(5) {
                    AgentState::Running
                } else {
                    AgentState::Starting
                }
            },
            
            // Agent was starting but service stopped
            (AgentState::Starting, false) => AgentState::Stopped,
            
            // Agent was running, service still up
            (AgentState::Running, true) => AgentState::Running,
            
            // Agent was running but service stopped
            (AgentState::Running, false) => AgentState::Stopped,
        };
        
        if new_agent_state != self.current_agent_state {
            eprintln!("Agent state: {:?} -> {:?}", self.current_agent_state, new_agent_state);
            self.current_agent_state = new_agent_state;
            self.agent_state_change_time = Instant::now();
        }
    }
    
    fn handle_metrics_success(&mut self, all_metrics: AllMetrics) {
        self.metrics_history.push(&all_metrics);
        self.current_all_metrics = Some(all_metrics.clone());
        self.error_count = 0;
        
        // Determine program state based on agent state and metrics
        self.current_program_state = self.determine_program_state_from_agent_and_models(&all_metrics);
    }
    
    fn handle_metrics_error(&mut self, error: Box<dyn Error>) {
        eprintln!("Metrics fetch failed: {}", error);
        self.error_count += 1;
        
        // Set program state based on agent state when metrics fail
        self.current_program_state = match self.current_agent_state {
            AgentState::Running => ProgramState::ServiceLoadedNoModel,
            AgentState::Starting => ProgramState::AgentStarting,
            AgentState::Stopped => ProgramState::AgentNotLoaded,
            AgentState::NotInstalled => ProgramState::AgentNotLoaded,
        };
        
        if !matches!(self.current_agent_state, AgentState::Running) {
            self.current_all_metrics = None;
            self.metrics_history.clear();
        } else {
            self.current_all_metrics = None;
        }
    }
    
    fn determine_program_state_from_agent_and_models(&self, all_metrics: &AllMetrics) -> ProgramState {
        // First check agent state
        match self.current_agent_state {
            AgentState::NotInstalled | AgentState::Stopped => ProgramState::AgentNotLoaded,
            AgentState::Starting => ProgramState::AgentStarting,
            AgentState::Running => {
                // Agent is running, check model states
                if all_metrics.models.is_empty() {
                    ProgramState::ServiceLoadedNoModel
                } else {
                    // Check if any models are loading
                    let has_loading_models = all_metrics.models.iter()
                        .any(|m| m.model_state == crate::models::ModelState::Loading);
                    
                    if has_loading_models {
                        ProgramState::ModelLoading
                    } else {
                        // Check for queue activity in running models
                        let has_queue_activity = all_metrics.models.iter()
                            .filter(|m| m.model_state == crate::models::ModelState::Running)
                            .any(|m| m.metrics.requests_processing > 0 || m.metrics.requests_deferred > 0);
                        
                        if has_queue_activity {
                            ProgramState::ModelProcessingQueue
                        } else {
                            ProgramState::ModelReady
                        }
                    }
                }
            }
        }
    }
}

fn main() {
    setup_panic_handler();
    
    if let Err(e) = run() {
        handle_error(e);
    }
}

fn setup_panic_handler() {
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("Plugin panic: {:?}", panic_info);
        if let Ok(menu) = menu::build_error_menu("Plugin encountered an error") {
            print!("{}", menu);
        }
    }));
}

fn handle_error(error: Box<dyn Error>) {
    eprintln!("Plugin error: {:?}", error);
    if let Ok(menu) = menu::build_error_menu(&format!("Error: {}", error)) {
        print!("{}", menu);
    }
    std::process::exit(1);
}

fn run() -> Result<()> {
    if let Some(command) = std::env::args().nth(1) {
        return commands::handle_command(&command);
    }

    let is_swiftbar = std::env::var("SWIFTBAR").is_ok();
    
    if constants::STREAMING_MODE && is_swiftbar {
        run_streaming_mode()
    } else {
        run_once()
    }
}

fn run_streaming_mode() -> Result<()> {
    let running = setup_shutdown_handler()?;
    let mut state = PluginState::new()?;
    
    eprintln!("Starting adaptive polling mode");
    
    while running.load(Ordering::SeqCst) {
        let loop_start = Instant::now();
        
        let frame = render_frame(&mut state)?;
        print!("~~~\n{}", frame);
        io::stdout().flush()?;
        
        let sleep_duration = Duration::from_secs(state.polling_mode.interval_secs());
        adaptive_sleep(sleep_duration, &running);
        
        log_slow_iteration(loop_start, &state);
    }
    
    eprintln!("Plugin shutting down gracefully");
    Ok(())
}

fn run_once() -> Result<()> {
    let mut state = PluginState::new()?;
    let frame = render_frame(&mut state)?;
    print!("{}", frame);
    Ok(())
}

fn render_frame(state: &mut PluginState) -> Result<String> {
    state.update_state();
    
    let menu_state = menu::PluginState {
        http_client: state.http_client.clone(),
        metrics_history: state.metrics_history.clone(),
        current_program_state: state.current_program_state,
        current_agent_state: state.current_agent_state,
        current_all_metrics: state.current_all_metrics.clone(),
        error_count: state.error_count,
    };
    
    menu::build_menu(&menu_state)
}

fn setup_shutdown_handler() -> Result<Arc<AtomicBool>> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })?;
    
    Ok(running)
}

fn adaptive_sleep(duration: Duration, running: &Arc<AtomicBool>) {
    let sleep_chunks = duration.as_secs().max(1);
    let chunk_duration = Duration::from_secs(1);
    
    for _ in 0..sleep_chunks {
        if !running.load(Ordering::SeqCst) {
            break;
        }
        thread::sleep(chunk_duration);
    }
    
    let remainder = duration - Duration::from_secs(sleep_chunks);
    if remainder > Duration::ZERO && running.load(Ordering::SeqCst) {
        thread::sleep(remainder);
    }
}

fn log_slow_iteration(loop_start: Instant, state: &PluginState) {
    if cfg!(debug_assertions) {
        let loop_duration = loop_start.elapsed();
        if loop_duration > Duration::from_millis(500) {
            eprintln!("Slow loop iteration: {:?} (mode: {})", 
                loop_duration, state.polling_mode.description());
        }
    }
}