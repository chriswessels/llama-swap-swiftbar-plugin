mod charts;
mod commands;
mod constants;
mod icons;
mod menu;
mod metrics;
mod models;
mod service;
mod state_model;
mod types;

// All imports are now handled in types.rs
use crate::types::{PluginState, Result};
use std::error::Error;
use std::io::{self, Write};
// Removed AtomicBool import as we now use channels for shutdown signaling
use std::sync::mpsc;
// Removed thread import as adaptive_sleep no longer uses thread::sleep
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
        eprintln!("Plugin panic: {panic_info:?}");
        if let Ok(menu) = menu::build_error_menu("Plugin encountered an error") {
            print!("{menu}");
        }
    }));
}

fn handle_error(error: Box<dyn Error>) {
    eprintln!("Plugin error: {error:?}");
    if let Ok(menu) = menu::build_error_menu(&format!("Error: {error}")) {
        print!("{menu}");
    }
    std::process::exit(1);
}

fn run() -> Result<()> {
    if let Some(command) = std::env::args().nth(1) {
        return commands::handle_command(&command);
    }

    let is_swiftbar = std::env::var("SWIFTBAR").is_ok();

    if *constants::STREAMING_MODE && is_swiftbar {
        run_streaming_mode()
    } else {
        run_once()
    }
}

fn run_streaming_mode() -> Result<()> {
    let shutdown_rx = setup_shutdown_handler()?;
    let mut state = PluginState::new()?;

    eprintln!("Starting adaptive polling mode");

    loop {
        let loop_start = Instant::now();

        let frame = render_frame(&mut state)?;
        print!("~~~\n{frame}");
        io::stdout().flush()?;

        let sleep_duration = state.polling_mode.interval();
        adaptive_sleep(sleep_duration, &shutdown_rx);

        // Check if we received a shutdown signal during sleep
        if shutdown_rx.try_recv().is_ok() {
            break;
        }

        log_slow_iteration(loop_start, &state);
    }

    eprintln!("Plugin shutting down gracefully");
    Ok(())
}

fn run_once() -> Result<()> {
    let mut state = PluginState::new()?;
    let frame = render_frame(&mut state)?;
    print!("{frame}");
    Ok(())
}

fn render_frame(state: &mut PluginState) -> Result<String> {
    state.update_state();
    menu::build_menu(state)
}

fn setup_shutdown_handler() -> Result<mpsc::Receiver<()>> {
    let (tx, rx) = mpsc::channel();

    ctrlc::set_handler(move || {
        let _ = tx.send(()); // Ignore send errors if receiver is dropped
    })?;

    Ok(rx)
}

fn adaptive_sleep(duration: Duration, shutdown_rx: &mpsc::Receiver<()>) {
    let _ = shutdown_rx.recv_timeout(duration);
    // If recv_timeout returns Ok(()), we got a shutdown signal
    // If it returns Err(RecvTimeoutError::Timeout), the duration elapsed
    // If it returns Err(RecvTimeoutError::Disconnected), the sender was dropped (also treat as shutdown)
    // In all cases, we just return - the caller will check if shutdown was requested
}

fn log_slow_iteration(loop_start: Instant, state: &PluginState) {
    if cfg!(debug_assertions) {
        let loop_duration = loop_start.elapsed();
        if loop_duration > Duration::from_millis(500) {
            eprintln!(
                "Slow loop iteration: {:?} (mode: {})",
                loop_duration,
                state.polling_mode.description()
            );
        }
    }
}
