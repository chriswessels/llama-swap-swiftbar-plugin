# Phase 2: Streaming Infrastructure Specification

## Overview

This phase implements the core streaming loop that enables the plugin to run continuously and push updates to SwiftBar. This is the foundation for real-time monitoring.

## Goals

- Implement the main streaming loop with proper timing
- Handle SwiftBar's streaming protocol (~~~ delimiters)
- Ensure proper stdout flushing for immediate updates
- Add graceful error handling and recovery

## Key Concepts

### SwiftBar Streaming Mode

SwiftBar supports "streamable" plugins that:
1. Run as a single long-lived process
2. Output menu updates separated by `~~~` on its own line
3. Each output block completely replaces the previous menu

### Output Format

```
[First menu output]
~~~
[Second menu output]
~~~
[Third menu output]
...
```

## Implementation

### 2.1 State Management Structure

First, create a state container in src/main.rs:

```rust
use crate::models::{MetricsHistory, ServiceStatus};
use reqwest::blocking::Client;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

struct PluginState {
    http_client: Client,
    metrics_history: MetricsHistory,
    current_status: ServiceStatus,
    is_first_iteration: bool,
}

impl PluginState {
    fn new() -> Result<Self> {
        // Create HTTP client with timeout
        let http_client = Client::builder()
            .timeout(Duration::from_secs(constants::API_TIMEOUT_SECS))
            .build()?;

        Ok(Self {
            http_client,
            metrics_history: MetricsHistory::new(),
            current_status: ServiceStatus::Unknown,
            is_first_iteration: true,
        })
    }
}
```

### 2.2 Main Streaming Loop

Update the `run_streaming_mode` function:

```rust
fn run_streaming_mode() -> Result<()> {
    // Initialize state
    let mut state = PluginState::new()?;
    
    // Lock stdout for efficiency
    let stdout = io::stdout();
    let mut stdout_handle = stdout.lock();
    
    // Main loop
    loop {
        // If not first iteration, print delimiter
        if !state.is_first_iteration {
            writeln!(stdout_handle, "~~~")?;
        } else {
            state.is_first_iteration = false;
        }
        
        // Update state (fetch metrics, check service status)
        update_state(&mut state);
        
        // Generate and output menu
        let menu = menu::build_menu(&state);
        write!(stdout_handle, "{}", menu)?;
        
        // Critical: flush to ensure SwiftBar sees the update
        stdout_handle.flush()?;
        
        // Sleep until next update
        thread::sleep(Duration::from_secs(constants::UPDATE_INTERVAL_SECS));
    }
}

fn update_state(state: &mut PluginState) {
    // Try to fetch metrics
    match metrics::fetch_metrics(&state.http_client) {
        Ok(metrics) => {
            state.current_status = ServiceStatus::Running;
            state.metrics_history.push(&metrics);
        }
        Err(_) => {
            // Service is likely down
            state.current_status = ServiceStatus::Stopped;
            // Don't update history when service is down
        }
    }
}
```

### 2.3 Single Execution Mode

For compatibility, implement non-streaming mode:

```rust
fn run_once() -> Result<()> {
    let mut state = PluginState::new()?;
    
    // Update state once
    update_state(&mut state);
    
    // Generate and output menu
    let menu = menu::build_menu(&state);
    print!("{}", menu);
    
    Ok(())
}
```

### 2.4 Error Handling Strategy

Create a robust error handling wrapper:

```rust
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
```

### 2.5 Graceful Shutdown

Add signal handling for clean shutdown:

```rust
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn run_streaming_mode() -> Result<()> {
    // Set up shutdown flag
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    
    // Handle Ctrl+C and termination signals
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })?;
    
    // Initialize state
    let mut state = PluginState::new()?;
    let stdout = io::stdout();
    let mut stdout_handle = stdout.lock();
    
    // Main loop with shutdown check
    while running.load(Ordering::SeqCst) {
        if !state.is_first_iteration {
            writeln!(stdout_handle, "~~~")?;
        } else {
            state.is_first_iteration = false;
        }
        
        update_state(&mut state);
        let menu = menu::build_menu(&state);
        write!(stdout_handle, "{}", menu)?;
        stdout_handle.flush()?;
        
        // Interruptible sleep
        for _ in 0..constants::UPDATE_INTERVAL_SECS {
            if !running.load(Ordering::SeqCst) {
                break;
            }
            thread::sleep(Duration::from_secs(1));
        }
    }
    
    // Clean shutdown
    eprintln!("Plugin shutting down gracefully");
    Ok(())
}
```

Add to Cargo.toml:
```toml
ctrlc = "3.4"
```

### 2.6 Menu Building Placeholder

Create src/menu.rs with placeholder implementation:

```rust
use bitbar::{Menu, MenuItem};
use crate::PluginState;

pub fn build_menu(state: &PluginState) -> Menu {
    let mut items = vec![];
    
    // Title with status
    let title = match state.current_status {
        crate::models::ServiceStatus::Running => "🟢 Running",
        crate::models::ServiceStatus::Stopped => "🔴 Stopped",
        crate::models::ServiceStatus::Unknown => "⚪ Unknown",
    };
    
    items.push(MenuItem::new(title));
    items.push(MenuItem::Sep);
    items.push(MenuItem::new("Llama-Swap SwiftBar Plugin"));
    items.push(MenuItem::new(format!("Status: {:?}", state.current_status)));
    
    Menu(items)
}

pub fn build_error_menu(message: &str) -> Result<Menu, std::fmt::Error> {
    Ok(Menu(vec![
        MenuItem::new("⚠️ Error"),
        MenuItem::Sep,
        MenuItem::new(message),
    ]))
}
```

### 2.7 Metrics Placeholder

Create src/metrics.rs with placeholder:

```rust
use reqwest::blocking::Client;
use crate::models::{Metrics, MetricsResponse};
use crate::constants;

pub fn fetch_metrics(client: &Client) -> crate::Result<Metrics> {
    let url = format!("{}:{}/metrics", constants::API_BASE_URL, constants::API_PORT);
    
    // For now, return dummy data or error
    // This will be implemented in Phase 4
    Err("Metrics not yet implemented".into())
}
```

### 2.8 Commands Placeholder

Create src/commands.rs:

```rust
pub fn handle_command(command: &str) -> crate::Result<()> {
    match command {
        "test" => {
            println!("Command handling works!");
            Ok(())
        }
        _ => Err(format!("Unknown command: {}", command).into()),
    }
}
```

## Testing the Streaming Infrastructure

### Manual Testing

1. Build the plugin:
   ```bash
   cargo build
   ```

2. Run in terminal to see output:
   ```bash
   ./target/debug/llama-swap-swiftbar
   ```

3. Verify:
   - Menu output appears
   - After 5 seconds, see `~~~` followed by new menu
   - Ctrl+C shuts down gracefully

### SwiftBar Testing

1. Build release version:
   ```bash
   cargo build --release
   ```

2. Copy to SwiftBar plugins directory:
   ```bash
   cp target/release/llama-swap-swiftbar ~/Library/Application\ Support/SwiftBar/Plugins/llama-swap.5s.o
   chmod +x ~/Library/Application\ Support/SwiftBar/Plugins/llama-swap.5s.o
   ```

3. Set streaming attribute:
   ```bash
   xattr -w com.ameba.SwiftBar.type streamable ~/Library/Application\ Support/SwiftBar/Plugins/llama-swap.5s.o
   ```

4. Refresh SwiftBar to load plugin

## Common Issues and Solutions

### Output Not Updating
- Ensure stdout is flushed after each menu output
- Check that `~~~` delimiter is on its own line
- Verify streaming attribute is set

### High CPU Usage
- Check sleep duration is correct
- Ensure no tight loops without delays
- Profile with `cargo flamegraph` if needed

### Plugin Crashes
- Check error handling covers all paths
- Use `RUST_BACKTRACE=1` for debugging
- Add logging to identify crash location

## Next Steps

With streaming infrastructure in place, proceed to [Phase 3: Visual Components](03-visual-components.md) to implement icons and charts.