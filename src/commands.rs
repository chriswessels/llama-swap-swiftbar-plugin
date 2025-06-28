use std::process::Command;
use crate::constants::LAUNCH_AGENT_LABEL;
use crate::types::error_helpers::{with_context, get_home_dir, CONNECT_API, START_SERVICE, STOP_SERVICE, GET_USER_ID, CREATE_DIR, CREATE_FILE, EXEC_COMMAND};


pub fn handle_command(command: &str) -> crate::Result<()> {
    match command {
        "do_start" => start_service(),
        "do_stop" => stop_service(),
        "do_restart" => restart_service(),
        "do_unload" => unload_models(),
        "do_install" => install_service(),
        "do_uninstall" => uninstall_service(),
        "view_logs" => view_file(crate::constants::LOG_FILE_PATH, create_default_log),
        "view_config" => view_file(crate::constants::CONFIG_FILE_PATH, create_default_config),
        _ => Err(format!("Unknown command: {command}").into()),
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
    let output = with_context(
        run_launchctl_command("kickstart", &["-kp", &service_context.service_target]),
        START_SERVICE
    )?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to start service: {stderr}").into());
    }
    
    eprintln!("Service started successfully");
    Ok(())
}

fn stop_service() -> crate::Result<()> {
    eprintln!("Stopping Llama-Swap service...");
    
    ensure_service_installed()?;
    let service_context = ServiceContext::new()?;
    
    let output = with_context(
        run_launchctl_command("bootout", &[&service_context.service_target]),
        STOP_SERVICE
    )?;
    
    // bootout can return non-zero if service wasn't running, but that's ok
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.contains("No such process") {
            return Err(format!("Failed to stop service: {stderr}").into());
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
    
    let response = with_context(
        client
            .get(&url)
            .timeout(std::time::Duration::from_secs(10))
            .send(),
        CONNECT_API
    )?;
    
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
    
    let output = with_context(
        Command::new("open")
            .args(["-t", &expanded_path])
            .output(),
        EXEC_COMMAND
    )?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to open file: {stderr}").into());
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
        let target_domain = format!("gui/{user_id}");
        let service_target = format!("{target_domain}/{LAUNCH_AGENT_LABEL}");
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
        with_context(std::fs::create_dir_all(parent), CREATE_DIR)?;
    }
    
    // Create file with default content
    with_context(std::fs::write(path, default_content_fn()), CREATE_FILE)?;
    
    Ok(())
}

fn run_launchctl_command(subcommand: &str, args: &[&str]) -> Result<std::process::Output, std::io::Error> {
    Command::new("launchctl")
        .arg(subcommand)
        .args(args)
        .output()
}

fn get_user_id() -> crate::Result<String> {
    let output = with_context(
        Command::new("id")
            .arg("-u")
            .output(),
        GET_USER_ID
    )?;
    
    if !output.status.success() {
        return Err("Failed to get user ID".into());
    }
    
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn is_service_installed() -> crate::Result<bool> {
    let plist_path = get_plist_path()?;
    Ok(std::path::Path::new(&plist_path).exists())
}

fn get_plist_path() -> crate::Result<String> {
    let home = get_home_dir()?;
    Ok(format!("{home}/Library/LaunchAgents/{LAUNCH_AGENT_LABEL}.plist"))
}

fn expand_tilde(path: &str) -> crate::Result<String> {
    if path.starts_with("~/") {
        let home = get_home_dir()?;
        Ok(path.replacen('~', &home, 1))
    } else {
        Ok(path.to_string())
    }
}

fn create_default_log() -> &'static str {
    "# Llama-Swap Plugin Log\n"
}

fn install_service() -> crate::Result<()> {
    if is_service_installed()? {
        return Err("Service already installed".into());
    }
    
    let binary_path = find_llama_swap_binary()?;
    let plist_content = generate_plist_content(&binary_path)?;
    let plist_path = get_plist_path()?;
    
    // Create LaunchAgents directory if it doesn't exist
    if let Some(parent) = std::path::Path::new(&plist_path).parent() {
        with_context(std::fs::create_dir_all(parent), CREATE_DIR)?;
    }
    
    // Write plist file
    with_context(std::fs::write(&plist_path, plist_content), CREATE_FILE)?;
    
    // Set proper permissions (644)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o644);
        with_context(std::fs::set_permissions(&plist_path, perms), "Failed to set plist permissions")?;
    }
    
    Ok(())
}

fn uninstall_service() -> crate::Result<()> {
    eprintln!("Uninstalling Llama-Swap service...");
    
    if !is_service_installed()? {
        return Err("Service is not installed.".into());
    }
    
    // Try to stop the service first if it's running
    if crate::service::is_service_running() {
        eprintln!("Stopping service before uninstallation...");
        let _ = stop_service(); // Continue even if stop fails
    }
    
    let plist_path = get_plist_path()?;
    
    // Remove plist file
    with_context(std::fs::remove_file(&plist_path), "Failed to remove plist file")?;
    
    eprintln!("Service uninstalled successfully");
    Ok(())
}

pub fn find_llama_swap_binary() -> crate::Result<String> {
    // Check if llama-swap is in PATH
    let output = Command::new("which")
        .arg("llama-swap")
        .output()
        .map_err(|_| "Failed to run 'which' command")?;
    
    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Ok(path);
        }
    }
    
    Err("llama-swap binary not found in PATH. Please install llama-swap first:\n\n  brew install llama-swap\n\nOr ensure it's available in your PATH.".into())
}


fn generate_plist_content(binary_path: &str) -> crate::Result<String> {
    let log_path = expand_tilde(crate::constants::LOG_FILE_PATH)?;
    let working_dir = get_home_dir()?;
    
    let plist = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
        <string>--port</string>
        <string>{}</string>
    </array>
    <key>WorkingDirectory</key>
    <string>{}</string>
    <key>RunAtLoad</key>
    <false/>
    <key>KeepAlive</key>
    <false/>
    <key>StandardOutPath</key>
    <string>{}</string>
    <key>StandardErrorPath</key>
    <string>{}</string>
</dict>
</plist>"#,
        LAUNCH_AGENT_LABEL,
        binary_path,
        crate::constants::API_PORT,
        working_dir,
        log_path,
        log_path
    );
    
    Ok(plist)
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