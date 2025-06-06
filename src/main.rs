mod constants;
mod models;
mod menu;
mod metrics;
mod charts;
mod icons;
mod commands;
mod service;

use crate::models::{AllMetricsHistory, ServiceStatus, AllMetrics};
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
    pub current_status: ServiceStatus,
    pub current_all_metrics: Option<AllMetrics>,
    pub error_count: usize,
    pub polling_mode: PollingMode,
    pub last_status: ServiceStatus,
    pub mode_change_time: Instant,
}

impl PluginState {
    fn new() -> Result<Self> {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(constants::API_TIMEOUT_SECS))
            .build()?;

        Ok(Self {
            http_client,
            metrics_history: AllMetricsHistory::new(),
            current_status: ServiceStatus::Unknown,
            current_all_metrics: None,
            error_count: 0,
            polling_mode: PollingMode::Starting,
            last_status: ServiceStatus::Unknown,
            mode_change_time: Instant::now(),
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
        // Status transition takes priority
        if self.current_status != self.last_status {
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
        match self.current_status {
            ServiceStatus::Running => PollingMode::Idle,
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
        if self.current_status != self.last_status {
            return format!("status changed: {:?} -> {:?}", self.last_status, self.current_status);
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
        self.last_status = self.current_status;
        
        match metrics::fetch_all_metrics(&self.http_client) {
            Ok(all_metrics) => self.handle_metrics_success(all_metrics),
            Err(e) => self.handle_metrics_error(e),
        }
        
        self.update_polling_mode();
    }
    
    fn handle_metrics_success(&mut self, all_metrics: AllMetrics) {
        self.current_status = ServiceStatus::Running;
        self.metrics_history.push(&all_metrics);
        self.current_all_metrics = Some(all_metrics);
        self.error_count = 0;
    }
    
    fn handle_metrics_error(&mut self, error: Box<dyn Error>) {
        eprintln!("Metrics fetch failed: {}", error);
        self.error_count += 1;
        
        self.current_status = if service::is_service_running(service::DetectionMethod::LaunchctlList) {
            eprintln!("Service is running but API is not responding");
            ServiceStatus::Running
        } else {
            ServiceStatus::Stopped
        };
        
        self.current_all_metrics = None;
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
        current_status: state.current_status,
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