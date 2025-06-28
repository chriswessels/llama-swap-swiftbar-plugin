mod constants;
mod models;
mod menu;
mod metrics;
mod charts;
mod icons;
mod commands;
mod service;
mod state_machines;
mod types;

// All imports are now handled in types.rs
use crate::types::{PluginState, Result};
use std::error::Error;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};



// PluginState is now in types.rs to avoid duplication

// PluginState implementation is now in types.rs

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
    menu::build_menu(state)
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