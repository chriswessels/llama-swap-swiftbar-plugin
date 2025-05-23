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
fn view_logs() -> crate::Result<()> {
    let log_path = expand_tilde(crate::constants::LOG_FILE_PATH)?;
    
    // Create the file if it doesn't exist
    if !std::path::Path::new(&log_path).exists() {
        // Create parent directory if needed
        if let Some(parent) = std::path::Path::new(&log_path).parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create log directory: {}", e))?;
        }
        
        // Create empty log file
        std::fs::write(&log_path, "# Llama-Swap Plugin Log\n")
            .map_err(|e| format!("Failed to create log file: {}", e))?;
    }
    
    let output = Command::new("open")
        .args(&["-t", &log_path])
        .output()
        .map_err(|e| format!("Failed to execute open command: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to open log file: {}", stderr).into());
    }
    
    Ok(())
}

/// Open config file in default text editor
fn view_config() -> crate::Result<()> {
    let config_path = expand_tilde(crate::constants::CONFIG_FILE_PATH)?;
    
    // Create the file if it doesn't exist
    if !std::path::Path::new(&config_path).exists() {
        // Create parent directory if needed
        if let Some(parent) = std::path::Path::new(&config_path).parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }
        
        // Create default config file
        let default_config = r#"# Llama-Swap Plugin Configuration
# Configuration for the SwiftBar plugin

# Service settings
service:
  url: "http://127.0.0.1:45786"
  timeout: 5

# Display settings
display:
  update_interval: 5
  show_metrics: true
  show_sparklines: true

# Monitoring settings
monitoring:
  history_size: 60
  alert_thresholds:
    memory_mb: 4096
    tps_low: 1.0
"#;
        
        std::fs::write(&config_path, default_config)
            .map_err(|e| format!("Failed to create config file: {}", e))?;
    }
    
    let output = Command::new("open")
        .args(&["-t", &config_path])
        .output()
        .map_err(|e| format!("Failed to execute open command: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to open config file: {}", stderr).into());
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