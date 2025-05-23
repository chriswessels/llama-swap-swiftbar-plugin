# Phase 4: Service Integration Specification

## Overview

This phase implements the integration with the Llama-Swap service, including status monitoring, control commands (start/stop/restart), and metrics fetching from the HTTP API.

## Goals

- Implement service status detection via API availability
- Create LaunchAgent control commands
- Build HTTP client for metrics API
- Handle file operations for logs and config viewing

## Implementation

### 4.1 Service Monitoring

Update src/metrics.rs with full implementation:

```rust
use reqwest::blocking::Client;
use crate::models::{Metrics, MetricsResponse};
use crate::constants;
use std::time::Duration;

/// Fetch metrics from the Llama-Swap API
pub fn fetch_metrics(client: &Client) -> crate::Result<Metrics> {
    let url = format!("{}:{}/metrics", constants::API_BASE_URL, constants::API_PORT);
    
    // Make HTTP request
    let response = client
        .get(&url)
        .send()
        .map_err(|e| format!("Failed to connect to API: {}", e))?;
    
    // Check status code
    if !response.status().is_success() {
        return Err(format!("API returned error: {}", response.status()).into());
    }
    
    // Parse JSON response
    let metrics_response: MetricsResponse = response
        .json()
        .map_err(|e| format!("Failed to parse metrics JSON: {}", e))?;
    
    // Convert to internal metrics format
    Ok(metrics_response.into())
}

/// Alternative: Check service status more explicitly
pub fn check_service_health(client: &Client) -> bool {
    let url = format!("{}:{}/health", constants::API_BASE_URL, constants::API_PORT);
    
    match client.get(&url).timeout(Duration::from_secs(1)).send() {
        Ok(response) => response.status().is_success(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_metrics_parsing() {
        // Test JSON parsing with sample data
        let json = r#"{
            "tps": 42.5,
            "memory_bytes": 1073741824,
            "cache_hits": 1000,
            "cache_misses": 50
        }"#;
        
        let response: MetricsResponse = serde_json::from_str(json).unwrap();
        let metrics: Metrics = response.into();
        
        assert_eq!(metrics.tps, 42.5);
        assert_eq!(metrics.memory_mb, 1024.0); // 1GB in MB
        assert_eq!(metrics.cache_hit_rate, 95.2380952); // 1000/(1000+50)*100
    }
}
```

### 4.2 LaunchAgent Control

Update src/commands.rs with service control implementation:

```rust
use std::process::Command;
use crate::constants::LAUNCH_AGENT_LABEL;

/// Handle command-line arguments from menu clicks
pub fn handle_command(command: &str) -> crate::Result<()> {
    match command {
        "do_start" => start_service(),
        "do_stop" => stop_service(),
        "do_restart" => restart_service(),
        "view_logs" => view_logs(),
        "view_config" => view_config(),
        _ => Err(format!("Unknown command: {}", command).into()),
    }
}

/// Start the Llama-Swap service
#[bitbar::command]
fn start_service() -> crate::Result<()> {
    eprintln!("Starting Llama-Swap service...");
    
    let output = Command::new("launchctl")
        .args(&["start", LAUNCH_AGENT_LABEL])
        .output()
        .map_err(|e| format!("Failed to execute launchctl: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to start service: {}", stderr).into());
    }
    
    eprintln!("Service start command sent successfully");
    Ok(())
}

/// Stop the Llama-Swap service
#[bitbar::command]
fn stop_service() -> crate::Result<()> {
    eprintln!("Stopping Llama-Swap service...");
    
    let output = Command::new("launchctl")
        .args(&["stop", LAUNCH_AGENT_LABEL])
        .output()
        .map_err(|e| format!("Failed to execute launchctl: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to stop service: {}", stderr).into());
    }
    
    eprintln!("Service stop command sent successfully");
    Ok(())
}

/// Restart the Llama-Swap service
#[bitbar::command]
fn restart_service() -> crate::Result<()> {
    eprintln!("Restarting Llama-Swap service...");
    
    // Try kickstart first (macOS 10.10+)
    let output = Command::new("launchctl")
        .args(&["kickstart", "-k", &format!("gui/{}/{}", get_user_id()?, LAUNCH_AGENT_LABEL)])
        .output();
    
    match output {
        Ok(result) if result.status.success() => {
            eprintln!("Service restarted successfully");
            return Ok(());
        }
        _ => {
            // Fallback to stop + start
            eprintln!("Kickstart failed, falling back to stop+start");
            stop_service()?;
            std::thread::sleep(std::time::Duration::from_millis(500));
            start_service()?;
        }
    }
    
    Ok(())
}

/// Get current user ID for launchctl commands
fn get_user_id() -> crate::Result<String> {
    let output = Command::new("id")
        .arg("-u")
        .output()
        .map_err(|e| format!("Failed to get user ID: {}", e))?;
    
    if !output.status.success() {
        return Err("Failed to get user ID".into());
    }
    
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Open log file in default text editor
#[bitbar::command]
fn view_logs() -> crate::Result<()> {
    let log_path = expand_tilde(crate::constants::LOG_FILE_PATH)?;
    
    let output = Command::new("open")
        .args(&["-t", &log_path])
        .output()
        .map_err(|e| format!("Failed to open log file: {}", e))?;
    
    if !output.status.success() {
        return Err("Failed to open log file".into());
    }
    
    Ok(())
}

/// Open config file in default text editor
#[bitbar::command]
fn view_config() -> crate::Result<()> {
    let config_path = expand_tilde(crate::constants::CONFIG_FILE_PATH)?;
    
    let output = Command::new("open")
        .args(&["-t", &config_path])
        .output()
        .map_err(|e| format!("Failed to open config file: {}", e))?;
    
    if !output.status.success() {
        return Err("Failed to open config file".into());
    }
    
    Ok(())
}

/// Expand ~ to user home directory
fn expand_tilde(path: &str) -> crate::Result<String> {
    if path.starts_with("~/") {
        let home = std::env::var("HOME")
            .map_err(|_| "Failed to get HOME directory")?;
        Ok(path.replacen("~", &home, 1))
    } else {
        Ok(path.to_string())
    }
}
```

### 4.3 Enhanced Menu Integration

Update the menu generation to include file operations:

```rust
// In src/menu.rs, update generate_control_items function:

fn generate_control_items(status: ServiceStatus) -> Vec<MenuItem> {
    let mut items = vec![];
    
    // Start/Stop based on status
    match status {
        ServiceStatus::Running => {
            items.push(
                MenuItem::new("🔴 Stop Llama-Swap")
                    .command(bitbar::Command::restart("do_stop"))
            );
        }
        ServiceStatus::Stopped | ServiceStatus::Unknown => {
            items.push(
                MenuItem::new("🟢 Start Llama-Swap")
                    .command(bitbar::Command::restart("do_start"))
            );
        }
    }
    
    // Always show restart
    items.push(
        MenuItem::new("⟲ Restart Llama-Swap")
            .command(bitbar::Command::restart("do_restart"))
    );
    
    items.push(MenuItem::Sep);
    
    // File operations
    items.push(
        MenuItem::new("📄 View Logs")
            .command(bitbar::Command::restart("view_logs"))
    );
    
    items.push(
        MenuItem::new("⚙️ View Config")
            .command(bitbar::Command::restart("view_config"))
    );
    
    items
}
```

### 4.4 Alternative Service Detection

For more robust service detection, implement multiple methods:

```rust
// In src/service.rs (new file)

use std::process::Command;
use crate::constants::LAUNCH_AGENT_LABEL;

pub enum DetectionMethod {
    ApiCheck,
    LaunchctlList,
    ProcessCheck,
}

/// Check if service is running using multiple methods
pub fn is_service_running(method: DetectionMethod) -> bool {
    match method {
        DetectionMethod::ApiCheck => {
            // Already implemented via metrics fetch
            true
        }
        DetectionMethod::LaunchctlList => check_via_launchctl(),
        DetectionMethod::ProcessCheck => check_via_ps(),
    }
}

/// Check service status via launchctl
fn check_via_launchctl() -> bool {
    let output = Command::new("launchctl")
        .args(&["list", LAUNCH_AGENT_LABEL])
        .output();
    
    match output {
        Ok(result) => {
            // launchctl list returns 0 if service is loaded
            if result.status.success() {
                // Parse output to check if actually running
                let output_str = String::from_utf8_lossy(&result.stdout);
                // Output format: PID Status Label
                let parts: Vec<&str> = output_str.split_whitespace().collect();
                if parts.len() >= 1 {
                    // First field is PID, "-" means not running
                    return parts[0] != "-";
                }
            }
            false
        }
        Err(_) => false,
    }
}

/// Check if process is running via ps
fn check_via_ps() -> bool {
    let output = Command::new("pgrep")
        .arg("-f")
        .arg("llama-swap")
        .output();
    
    match output {
        Ok(result) => result.status.success(),
        Err(_) => false,
    }
}
```

### 4.5 Error Recovery

Enhance the main loop to handle service state changes gracefully:

```rust
// In src/main.rs, enhance update_state function:

fn update_state(state: &mut PluginState) {
    // Primary check: try to fetch metrics
    match metrics::fetch_metrics(&state.http_client) {
        Ok(metrics) => {
            // Service is running and responsive
            state.current_status = ServiceStatus::Running;
            state.metrics_history.push(&metrics);
        }
        Err(e) => {
            eprintln!("Metrics fetch failed: {}", e);
            
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
```

### 4.6 Configuration Support

Add support for environment variable configuration:

```rust
// In src/constants.rs, add:

use std::env;

lazy_static::lazy_static! {
    pub static ref API_PORT_CONFIGURED: u16 = {
        env::var("LLAMA_SWAP_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(API_PORT)
    };
    
    pub static ref LAUNCH_AGENT_LABEL_CONFIGURED: String = {
        env::var("LLAMA_SWAP_LABEL")
            .unwrap_or_else(|_| LAUNCH_AGENT_LABEL.to_string())
    };
    
    pub static ref LOG_FILE_PATH_CONFIGURED: String = {
        env::var("LLAMA_SWAP_LOG")
            .unwrap_or_else(|_| LOG_FILE_PATH.to_string())
    };
}
```

Add to Cargo.toml:
```toml
lazy_static = "1.4"
```

## Testing Service Integration

### Manual Testing

1. **Test without service running:**
   ```bash
   cargo run
   # Should show "Stopped" status and Start button
   ```

2. **Start a mock API server:**
   ```bash
   # In another terminal
   python3 -m http.server 8080
   ```

3. **Create mock metrics endpoint:**
   ```python
   # save as mock_server.py
   from http.server import HTTPServer, BaseHTTPRequestHandler
   import json
   
   class Handler(BaseHTTPRequestHandler):
       def do_GET(self):
           if self.path == '/metrics':
               self.send_response(200)
               self.send_header('Content-Type', 'application/json')
               self.end_headers()
               data = {
                   'tps': 42.5,
                   'memory_bytes': 1073741824,
                   'cache_hits': 1000,
                   'cache_misses': 50
               }
               self.wfile.write(json.dumps(data).encode())
   
   HTTPServer(('', 8080), Handler).serve_forever()
   ```

4. **Test service commands:**
   ```bash
   # Test individual commands
   cargo run -- do_start
   cargo run -- do_stop
   cargo run -- view_logs
   ```

### Integration Testing

Create integration tests in tests/integration.rs:

```rust
#[test]
fn test_service_commands() {
    // Test that commands don't panic
    let commands = ["do_start", "do_stop", "do_restart", "view_logs", "view_config"];
    
    for cmd in &commands {
        // Commands might fail (service not installed), but shouldn't panic
        let _ = llama_swap_swiftbar::commands::handle_command(cmd);
    }
}
```

## Next Steps

With service integration complete, proceed to [Phase 5: Data Management](05-data-management.md) to implement metrics history and persistence.