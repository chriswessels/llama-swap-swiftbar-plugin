mod constants;
mod models;
mod menu;
mod metrics;
mod charts;
mod icons;
mod commands;
mod service;

use crate::models::{AllModelMetricsHistory, ServiceStatus, AllModelMetrics};
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
    Idle,       // Service running but no queue activity
    Active,     // Requests being processed
    Starting,   // Service state transitions
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
    pub metrics_history: AllModelMetricsHistory,
    pub current_status: ServiceStatus,
    pub current_all_metrics: Option<AllModelMetrics>,
    pub error_count: usize,
    pub polling_mode: PollingMode,
    pub last_status: ServiceStatus,
    pub mode_change_time: Instant,
}

impl PluginState {
    fn new() -> Result<Self> {
        // Create HTTP client with timeout
        let http_client = Client::builder()
            .timeout(Duration::from_secs(constants::API_TIMEOUT_SECS))
            .build()?;
        
        // Create new metrics history
        let metrics_history = AllModelMetricsHistory::new();

        Ok(Self {
            http_client,
            metrics_history,
            current_status: ServiceStatus::Unknown,
            current_all_metrics: None,
            error_count: 0,
            polling_mode: PollingMode::Starting, // Start with faster polling
            last_status: ServiceStatus::Unknown,
            mode_change_time: Instant::now(),
        })
    }
    
    /// Update polling mode based on current metrics and status
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
        // Handle service state transitions
        if self.current_status != self.last_status {
            return PollingMode::Starting;
        }
        
        // Stay in Starting mode for at least 10 seconds after state change
        if self.polling_mode == PollingMode::Starting && 
           self.mode_change_time.elapsed() < Duration::from_secs(constants::MIN_STARTING_DURATION_SECS) {
            return PollingMode::Starting;
        }
        
        // Check if service is actively processing requests
        if let Some(ref all_metrics) = self.current_all_metrics {
            for model_metrics in &all_metrics.models {
                if model_metrics.metrics.requests_processing > 0 || model_metrics.metrics.requests_deferred > 0 {
                    return PollingMode::Active;
                }
            }
        }
        
        // Default to idle when service is running but no queue activity
        if self.current_status == ServiceStatus::Running {
            PollingMode::Idle
        } else {
            PollingMode::Starting
        }
    }
    
    fn get_mode_reason(&self) -> String {
        if self.current_status != self.last_status {
            return format!("status changed: {:?} -> {:?}", self.last_status, self.current_status);
        }
        
        if let Some(ref all_metrics) = self.current_all_metrics {
            let mut total_processing = 0;
            let mut total_deferred = 0;
            for model_metrics in &all_metrics.models {
                total_processing += model_metrics.metrics.requests_processing;
                total_deferred += model_metrics.metrics.requests_deferred;
            }
            if total_processing > 0 {
                return format!("processing {} requests", total_processing);
            }
            if total_deferred > 0 {
                return format!("{} requests queued", total_deferred);
            }
        }
        
        "no queue activity".to_string()
    }
}

fn main() {
    // Set up panic handler to avoid crashes
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("Plugin panic: {:?}", panic_info);
        // Try to output an error menu before exiting
        if let Ok(menu) = menu::build_error_menu("Plugin encountered an error") {
            print!("{}", menu);
        }
    }));

    // Run with error recovery
    if let Err(e) = run() {
        eprintln!("Plugin error: {:?}", e);
        // Output error menu
        if let Ok(menu) = menu::build_error_menu(&format!("Error: {}", e)) {
            print!("{}", menu);
        }
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    // Check if running as a command (for menu item clicks)
    if let Some(command) = std::env::args().nth(1) {
        return commands::handle_command(&command);
    }

    // Detect if we should run in streaming mode
    // SwiftBar sets SWIFTBAR=1 in environment
    let is_swiftbar = std::env::var("SWIFTBAR").is_ok();
    
    if constants::STREAMING_MODE && is_swiftbar {
        run_streaming_mode()
    } else {
        run_once()
    }
}

fn render_frame(state: &mut PluginState) -> Result<String> {
    update_state(state);
    
    // Convert to menu PluginState
    let menu_state = menu::PluginState {
        http_client: state.http_client.clone(),
        metrics_history: state.metrics_history.clone(),
        current_status: state.current_status,
        current_all_metrics: state.current_all_metrics.clone(),
        error_count: state.error_count,
    };
    
    menu::build_menu(&menu_state)
}

fn run_streaming_mode() -> Result<()> {
    // Set up shutdown flag
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    
    // Handle Ctrl+C and termination signals
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })?;
    
    // Initialize state and output
    let mut state = PluginState::new()?;
    
    eprintln!("Starting adaptive polling mode");
    
    // Main loop with adaptive timing
    while running.load(Ordering::SeqCst) {
        let loop_start = Instant::now();
        
        // Render current frame
        let frame = render_frame(&mut state)?;
        
        print!("~~~\n{}", frame);
        io::stdout().flush()?;
        
        // Determine sleep duration based on current mode
        let sleep_duration = Duration::from_secs(state.polling_mode.interval_secs());
        
        // Adaptive interruptible sleep
        adaptive_sleep(sleep_duration, &running);
        
        // Optional: Log timing for debugging
        if cfg!(debug_assertions) {
            let loop_duration = loop_start.elapsed();
            if loop_duration > Duration::from_millis(500) {
                eprintln!("Slow loop iteration: {:?} (mode: {})", 
                    loop_duration, state.polling_mode.description());
            }
        }
    }
    
    eprintln!("Plugin shutting down gracefully");
    Ok(())
}

/// Interruptible sleep that respects shutdown signal
fn adaptive_sleep(duration: Duration, running: &Arc<AtomicBool>) {
    let sleep_chunks = duration.as_secs().max(1); // At least 1 second chunks
    let chunk_duration = Duration::from_secs(1);
    
    for _ in 0..sleep_chunks {
        if !running.load(Ordering::SeqCst) {
            break;
        }
        thread::sleep(chunk_duration);
    }
    
    // Handle sub-second remainder
    let remainder = duration - Duration::from_secs(sleep_chunks);
    if remainder > Duration::ZERO && running.load(Ordering::SeqCst) {
        thread::sleep(remainder);
    }
}

fn run_once() -> Result<()> {
    let mut state = PluginState::new()?;
    let frame = render_frame(&mut state)?;
    print!("{}", frame);
    Ok(())
}

fn update_state(state: &mut PluginState) {
    // Store previous status for comparison
    state.last_status = state.current_status;
    
    // Primary check: try to fetch metrics
    match metrics::fetch_all_model_metrics(&state.http_client) {
        Ok(all_metrics) => {
            // Service is running and responsive
            state.current_status = ServiceStatus::Running;
            state.metrics_history.push(&all_metrics);
            state.current_all_metrics = Some(all_metrics);
            state.error_count = 0; // Reset error count on success
        }
        Err(e) => {
            eprintln!("Metrics fetch failed: {}", e);
            state.error_count += 1;
            
            // Secondary check: is service actually running?
            if service::is_service_running(service::DetectionMethod::LaunchctlList) {
                // Service is running but API is not responsive
                state.current_status = ServiceStatus::Running;
                eprintln!("Service is running but API is not responding");
            } else {
                // Service is truly stopped
                state.current_status = ServiceStatus::Stopped;
            }
            
            // Clear metrics when service is not responsive
            state.current_all_metrics = None;
        }
    }
    
    // Update polling mode based on new state
    state.update_polling_mode();
}
