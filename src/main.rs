mod constants;
mod models;
mod menu;
mod metrics;
mod charts;
mod icons;
mod commands;
mod service;

use crate::models::{MetricsHistory, ServiceStatus};
use reqwest::blocking::Client;
use std::error::Error;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub struct PluginState {
    pub http_client: Client,
    pub metrics_history: MetricsHistory,
    pub current_status: ServiceStatus,
    pub current_metrics: Option<models::Metrics>,
    pub error_count: usize,
}

impl PluginState {
    fn new() -> Result<Self> {
        // Create HTTP client with timeout
        let http_client = Client::builder()
            .timeout(Duration::from_secs(constants::API_TIMEOUT_SECS))
            .build()?;
        
        // Create new metrics history
        let metrics_history = MetricsHistory::new();

        Ok(Self {
            http_client,
            metrics_history,
            current_status: ServiceStatus::Unknown,
            current_metrics: None,
            error_count: 0,
        })
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
        current_metrics: state.current_metrics.clone(),
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
    let mut first_frame = true;
    
    // Main loop with shutdown check
    while running.load(Ordering::SeqCst) {
        let frame = render_frame(&mut state)?;
        
        if first_frame {
            print!("{}", frame);
            first_frame = false;
        } else {
            print!("~~~\n{}", frame);
        }
        io::stdout().flush()?;
        
        // Interruptible sleep
        for _ in 0..constants::UPDATE_INTERVAL_SECS {
            if !running.load(Ordering::SeqCst) {
                break;
            }
            thread::sleep(Duration::from_secs(1));
        }
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

fn update_state(state: &mut PluginState) {
    // Primary check: try to fetch metrics
    match metrics::fetch_metrics(&state.http_client) {
        Ok(metrics) => {
            // Service is running and responsive
            state.current_status = ServiceStatus::Running;
            state.metrics_history.push(&metrics);
            state.current_metrics = Some(metrics);
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
            
            // Don't update history when we can't get real data
        }
    }
}
