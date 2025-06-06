use std::process::Command;
use crate::constants::LAUNCH_AGENT_LABEL;

pub fn handle_command(command: &str) -> crate::Result<()> {
    match command {
        "do_start" => start_service(),
        "do_stop" => stop_service(),
        "do_restart" => restart_service(),
        "do_unload" => unload_models(),
        "view_logs" => view_file(crate::constants::LOG_FILE_PATH, create_default_log),
        "view_config" => view_file(crate::constants::CONFIG_FILE_PATH, create_default_config),
        _ => Err(format!("Unknown command: {}", command).into()),
    }
}

fn start_service() -> crate::Result<()> {
    eprintln!("Starting Llama-Swap service...");
    
    ensure_service_installed()?;
    let service_context = ServiceContext::new()?;
    
    // Run the setup commands
    let _ = run_launchctl_command("enable", &[&service_context.service_target]);
    let _ = run_launchctl_command("bootstrap", &[&service_context.target_domain, &service_context.plist_path]);
    
    // The final kickstart command is critical
    let output = run_launchctl_command("kickstart", &["-kp", &service_context.service_target])
        .map_err(|e| format!("Failed to start service: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to start service: {}", stderr).into());
    }
    
    eprintln!("Service started successfully");
    Ok(())
}

fn stop_service() -> crate::Result<()> {
    eprintln!("Stopping Llama-Swap service...");
    
    ensure_service_installed()?;
    let service_context = ServiceContext::new()?;
    
    let output = run_launchctl_command("bootout", &[&service_context.service_target])
        .map_err(|e| format!("Failed to stop service: {}", e))?;
    
    // bootout can return non-zero if service wasn't running, but that's ok
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.contains("No such process") {
            return Err(format!("Failed to stop service: {}", stderr).into());
        }
    }
    
    eprintln!("Service stopped successfully");
    Ok(())
}

fn restart_service() -> crate::Result<()> {
    eprintln!("Restarting Llama-Swap service...");
    
    ensure_service_installed()?;
    stop_service()?;
    start_service()
}

fn unload_models() -> crate::Result<()> {
    eprintln!("Unloading models...");
    
    let client = reqwest::blocking::Client::new();
    let url = format!("{}:{}/unload", crate::constants::API_BASE_URL, crate::constants::API_PORT);
    
    let response = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .map_err(|e| format!("Failed to connect to API: {}", e))?;
    
    if response.status().is_success() {
        eprintln!("Models unloaded successfully");
        Ok(())
    } else {
        Err(format!("Failed to unload models: {}", response.status()).into())
    }
}

fn view_file(file_path: &str, default_content_fn: fn() -> &'static str) -> crate::Result<()> {
    let expanded_path = expand_tilde(file_path)?;
    
    ensure_file_exists(&expanded_path, default_content_fn)?;
    
    let output = Command::new("open")
        .args(&["-t", &expanded_path])
        .output()
        .map_err(|e| format!("Failed to execute open command: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to open file: {}", stderr).into());
    }
    
    Ok(())
}

// Helper structs and functions

struct ServiceContext {
    target_domain: String,
    service_target: String,
    plist_path: String,
}

impl ServiceContext {
    fn new() -> crate::Result<Self> {
        let user_id = get_user_id()?;
        let target_domain = format!("gui/{}", user_id);
        let service_target = format!("{}/{}", target_domain, LAUNCH_AGENT_LABEL);
        let plist_path = get_plist_path()?;
        
        Ok(Self {
            target_domain,
            service_target,
            plist_path,
        })
    }
}

fn ensure_service_installed() -> crate::Result<()> {
    if !is_service_installed()? {
        return Err("Service is not installed. Please install the Llama-Swap launch agent first.".into());
    }
    Ok(())
}

fn ensure_file_exists(path: &str, default_content_fn: fn() -> &'static str) -> crate::Result<()> {
    if std::path::Path::new(path).exists() {
        return Ok(());
    }
    
    // Create parent directory if needed
    if let Some(parent) = std::path::Path::new(path).parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }
    
    // Create file with default content
    std::fs::write(path, default_content_fn())
        .map_err(|e| format!("Failed to create file: {}", e))?;
    
    Ok(())
}

fn run_launchctl_command(subcommand: &str, args: &[&str]) -> Result<std::process::Output, std::io::Error> {
    Command::new("launchctl")
        .arg(subcommand)
        .args(args)
        .output()
}

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

fn is_service_installed() -> crate::Result<bool> {
    let plist_path = get_plist_path()?;
    Ok(std::path::Path::new(&plist_path).exists())
}

fn get_plist_path() -> crate::Result<String> {
    let home = std::env::var("HOME")
        .map_err(|_| "Failed to get HOME directory")?;
    Ok(format!("{}/Library/LaunchAgents/{}.plist", home, LAUNCH_AGENT_LABEL))
}

fn expand_tilde(path: &str) -> crate::Result<String> {
    if path.starts_with("~/") {
        let home = std::env::var("HOME")
            .map_err(|_| "Failed to get HOME directory")?;
        Ok(path.replacen("~", &home, 1))
    } else {
        Ok(path.to_string())
    }
}

fn create_default_log() -> &'static str {
    "# Llama-Swap Plugin Log\n"
}

fn create_default_config() -> &'static str {
    r#"# Llama-Swap Plugin Configuration
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
"#
}