mod constants;
mod models;
mod menu;
mod metrics;
mod charts;
mod icons;
mod commands;
mod service;
mod state_machines;

use crate::models::{AllMetricsHistory, AllMetrics};
use crate::state_machines::agent::{AgentStateMachine, AgentStates, AgentEvents, AgentContext, ServiceRunning};
use crate::state_machines::model::{ModelStateMachine, ModelEvents, ModelContext, ModelLoading, ModelActive};
use crate::state_machines::polling_mode::{PollingModeStateMachine, PollingModeEvents, PollingModeContext, StateChange, QueueActivity};
use crate::state_machines::program::{ProgramStateMachine, ProgramStates, ProgramEvents, ProgramContext, AgentUpdate, ModelUpdate};
use reqwest::blocking::Client;
use std::collections::HashMap;
use std::error::Error;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

type Result<T> = std::result::Result<T, Box<dyn Error>>;


pub struct PluginState {
    pub http_client: Client,
    pub metrics_history: AllMetricsHistory,
    pub current_all_metrics: Option<AllMetrics>,
    pub error_count: usize,
    
    // State machines
    pub agent_state_machine: AgentStateMachine<AgentContext>,
    pub polling_mode_state_machine: PollingModeStateMachine<PollingModeContext>,
    pub program_state_machine: ProgramStateMachine<ProgramContext>,
    pub model_state_machines: HashMap<String, ModelStateMachine<ModelContext>>,
    
    // Previous states for change detection
    pub last_program_state: ProgramStates,
    pub last_agent_state: AgentStates,
}

impl PluginState {
    fn new() -> Result<Self> {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(constants::API_TIMEOUT_SECS))
            .build()?;

        let agent_context = AgentContext::new();
        let polling_mode_context = PollingModeContext::new();
        let program_context = ProgramContext::new();
        
        let agent_state_machine = AgentStateMachine::new(agent_context);
        let polling_mode_state_machine = PollingModeStateMachine::new(polling_mode_context);
        let program_state_machine = ProgramStateMachine::new(program_context);

        Ok(Self {
            http_client,
            metrics_history: AllMetricsHistory::new(),
            current_all_metrics: None,
            error_count: 0,
            agent_state_machine,
            polling_mode_state_machine,
            program_state_machine,
            model_state_machines: HashMap::new(),
            last_program_state: ProgramStates::AgentNotLoaded,
            last_agent_state: AgentStates::NotInstalled,
        })
    }
    
    fn update_polling_mode(&mut self) {
        let old_state = self.polling_mode_state_machine.state().clone();
        
        // Check for state changes that should trigger polling mode updates
        let current_program_state = self.program_state_machine.state().clone();
        let current_agent_state = self.agent_state_machine.state().clone();
        
        if current_program_state != self.last_program_state || current_agent_state != self.last_agent_state {
            let _ = self.polling_mode_state_machine.process_event(PollingModeEvents::StateChangeDetected(StateChange));
        }
        
        // Check for queue activity
        let has_activity = self.has_queue_activity();
        let _ = self.polling_mode_state_machine.process_event(PollingModeEvents::ActivityCheck(QueueActivity(has_activity)));
        
        let new_state = self.polling_mode_state_machine.state().clone();
        if new_state != old_state {
            eprintln!("Polling mode: {} -> {} ({})", 
                old_state.description(),
                new_state.description(),
                self.get_mode_reason()
            );
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
        let current_program_state = self.program_state_machine.state().clone();
        if current_program_state != self.last_program_state {
            return format!("program state changed: {:?} -> {:?}", self.last_program_state, current_program_state);
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
        self.last_program_state = self.program_state_machine.state().clone();
        self.last_agent_state = self.agent_state_machine.state().clone();
        
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
        let old_state = self.agent_state_machine.state().clone();
        
        // Send service detection event to state machine
        let _ = self.agent_state_machine.process_event(AgentEvents::ServiceDetected(ServiceRunning(is_service_running)));
        
        // Check if we need to complete startup after timeout
        if let AgentStates::Starting = self.agent_state_machine.state() {
            if self.agent_state_machine.context().should_complete_startup() {
                let _ = self.agent_state_machine.process_event(AgentEvents::StartupComplete(crate::state_machines::agent::StartupTimeout));
            }
        }
        
        let new_state = self.agent_state_machine.state().clone();
        if new_state != old_state {
            eprintln!("Agent state: {:?} -> {:?}", old_state, new_state);
        }
    }
    
    fn handle_metrics_success(&mut self, all_metrics: AllMetrics) {
        self.metrics_history.push(&all_metrics);
        self.current_all_metrics = Some(all_metrics.clone());
        self.error_count = 0;
        
        // Update model state machines
        self.update_model_state_machines(&all_metrics);
        
        // Update program state machine with agent and model updates
        self.update_program_state_machine(&all_metrics);
    }
    
    fn handle_metrics_error(&mut self, error: Box<dyn Error>) {
        eprintln!("Metrics fetch failed: {}", error);
        self.error_count += 1;
        
        // Clear model state machines on error
        self.model_state_machines.clear();
        
        // Update program state machine with agent state only (no models)
        let agent_state = self.agent_state_machine.state().clone();
        let _ = self.program_state_machine.process_event(ProgramEvents::AgentStateChanged(AgentUpdate(agent_state)));
        
        // Update with empty model state
        let model_update = ModelUpdate {
            has_models: false,
            has_loading: false,
            has_activity: false,
        };
        let _ = self.program_state_machine.process_event(ProgramEvents::ModelStateChanged(model_update));
        
        if !matches!(self.agent_state_machine.state(), AgentStates::Running) {
            self.current_all_metrics = None;
            self.metrics_history.clear();
        } else {
            self.current_all_metrics = None;
        }
    }
    
    fn update_model_state_machines(&mut self, all_metrics: &AllMetrics) {
        // Remove models that no longer exist
        let current_model_names: std::collections::HashSet<String> = all_metrics.models.iter().map(|m| m.model_name.clone()).collect();
        self.model_state_machines.retain(|name, _| current_model_names.contains(name));
        
        // Update or create state machines for each model
        for model_data in &all_metrics.models {
            let state_machine = self.model_state_machines.entry(model_data.model_name.clone())
                .or_insert_with(|| ModelStateMachine::new(ModelContext::new()));
            
            // Update with loading status
            let is_loading = model_data.model_state == crate::models::ModelState::Loading;
            let _ = state_machine.process_event(ModelEvents::LoadingUpdate(ModelLoading(is_loading)));
            
            // Update with active status
            let is_active = model_data.model_state == crate::models::ModelState::Running;
            let _ = state_machine.process_event(ModelEvents::ActiveUpdate(ModelActive(is_active)));
        }
    }
    
    fn update_program_state_machine(&mut self, all_metrics: &AllMetrics) {
        // Update with agent state
        let agent_state = self.agent_state_machine.state().clone();
        let _ = self.program_state_machine.process_event(ProgramEvents::AgentStateChanged(AgentUpdate(agent_state)));
        
        // Determine model summary
        let has_models = !all_metrics.models.is_empty();
        let has_loading = self.model_state_machines.values().any(|sm| sm.state().is_loading());
        let has_activity = self.has_queue_activity();
        
        let model_update = ModelUpdate {
            has_models,
            has_loading,
            has_activity,
        };
        
        let _ = self.program_state_machine.process_event(ProgramEvents::ModelStateChanged(model_update));
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
        
        let sleep_duration = Duration::from_secs(state.polling_mode_state_machine.state().interval_secs());
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
        current_program_state: state.program_state_machine.state().clone(),
        current_agent_state: state.agent_state_machine.state().clone(),
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
                loop_duration, state.polling_mode_state_machine.state().description());
        }
    }
}