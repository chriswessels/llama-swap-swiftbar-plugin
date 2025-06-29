use crate::constants::LAUNCH_AGENT_LABEL;
use crate::types::error_helpers::{
    get_home_dir, with_context, CONNECT_API, CREATE_DIR, CREATE_FILE, EXEC_COMMAND, GET_USER_ID,
    START_SERVICE, STOP_SERVICE,
};
use std::process::Command;

pub fn handle_command(command: &str) -> crate::Result<()> {
    match command {
        "do_start" => start_service(),
        "do_stop" => stop_service(),
        "do_restart" => restart_service(),
        "do_unload" => unload_models(),
        "do_install" => install_service(),
        "do_uninstall" => uninstall_service(),
        "open_ui" => open_ui(),
        "view_logs" => view_file(crate::constants::LOG_FILE_PATH, create_default_log),
        "view_config" => view_file(crate::constants::CONFIG_FILE_PATH, create_default_config),
        _ => Err(format!("Unknown command: {command}").into()),
    }
}

fn start_service() -> crate::Result<()> {
    eprintln!("Starting Llama-Swap service...");

    ensure_service_installed()?;
    let service_context = ServiceContext::new()?;

    // Enable the service (safe to run multiple times)
    let _ = run_launchctl_command("enable", &[&service_context.service_target]);

    // Only bootstrap if not already loaded
    if !crate::service::is_service_loaded() {
        let bootstrap_output = run_launchctl_command(
            "bootstrap",
            &[&service_context.target_domain, &service_context.plist_path],
        );
        if let Ok(output) = bootstrap_output {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!("Bootstrap warning: {stderr}");
            }
        }
    }

    // Kickstart the service (this actually starts it)
    let output = with_context(
        run_launchctl_command("kickstart", &["-kp", &service_context.service_target]),
        START_SERVICE,
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
        STOP_SERVICE,
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
    let service_context = ServiceContext::new()?;

    // Use kickstart -k to kill and restart the service atomically
    let output = with_context(
        run_launchctl_command("kickstart", &["-k", &service_context.service_target]),
        "restart service",
    )?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to restart service: {stderr}").into());
    }

    eprintln!("Service restarted successfully");
    Ok(())
}

fn unload_models() -> crate::Result<()> {
    eprintln!("Unloading models...");

    let client = reqwest::blocking::Client::new();
    let url = format!(
        "{}:{}/unload",
        crate::constants::API_BASE_URL,
        crate::constants::API_PORT
    );

    let response = with_context(
        client
            .get(&url)
            .timeout(std::time::Duration::from_secs(10))
            .send(),
        CONNECT_API,
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
        Command::new("open").args(["-t", &expanded_path]).output(),
        EXEC_COMMAND,
    )?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to open file: {stderr}").into());
    }

    Ok(())
}

fn open_ui() -> crate::Result<()> {
    let ui_url = format!("{}:{}/ui/models", crate::constants::API_BASE_URL, crate::constants::API_PORT);

    let output = with_context(Command::new("open").arg(ui_url).output(), EXEC_COMMAND)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to open UI: {stderr}").into());
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
        return Err(
            "Service is not installed. Please install the Llama-Swap launch agent first.".into(),
        );
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

fn run_launchctl_command(
    subcommand: &str,
    args: &[&str],
) -> Result<std::process::Output, std::io::Error> {
    Command::new("launchctl")
        .arg(subcommand)
        .args(args)
        .output()
}

fn get_user_id() -> crate::Result<String> {
    let output = with_context(Command::new("id").arg("-u").output(), GET_USER_ID)?;

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
    Ok(format!(
        "{home}/Library/LaunchAgents/{LAUNCH_AGENT_LABEL}.plist"
    ))
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
    eprintln!("Installing Llama-Swap service...");

    let binary_path = find_llama_swap_binary()?;
    let plist_content = generate_plist_content(&binary_path)?;
    let plist_path = get_plist_path()?;
    let service_context = ServiceContext::new()?;

    // If service is already loaded, unload it first to refresh the plist
    if crate::service::is_service_loaded() {
        eprintln!("Unloading existing service to refresh plist...");
        let _ = run_launchctl_command("bootout", &[&service_context.service_target]);
    }

    // Create LaunchAgents directory if it doesn't exist
    if let Some(parent) = std::path::Path::new(&plist_path).parent() {
        with_context(std::fs::create_dir_all(parent), CREATE_DIR)?;
    }

    // Write plist file (overwrite if exists)
    with_context(std::fs::write(&plist_path, plist_content), CREATE_FILE)?;

    // Set proper permissions (644)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o644);
        with_context(
            std::fs::set_permissions(&plist_path, perms),
            "Failed to set plist permissions",
        )?;
    }

    eprintln!("Service plist installed successfully");
    Ok(())
}

fn uninstall_service() -> crate::Result<()> {
    eprintln!("Uninstalling Llama-Swap service...");

    let service_context = ServiceContext::new()?;

    // Stop and unload from launchctl first
    if crate::service::is_service_loaded() {
        eprintln!("Unloading service from launchctl...");
        let _ = run_launchctl_command("bootout", &[&service_context.service_target]);
    }

    let plist_path = get_plist_path()?;

    // Remove plist file if it exists
    if std::path::Path::new(&plist_path).exists() {
        with_context(
            std::fs::remove_file(&plist_path),
            "Failed to remove plist file",
        )?;
        eprintln!("Service uninstalled successfully");
    } else {
        eprintln!("Service plist not found (already uninstalled)");
    }

    Ok(())
}

pub fn find_llama_swap_binary() -> crate::Result<String> {
    // Run which in a shell context to load user PATH configs
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    let output = Command::new(&shell)
        .args(["-i", "-c", "which llama-swap"]) // -i = interactive shell (loads .zshrc)
        .output()
        .map_err(|_| "Failed to run which command in shell context")?;

    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Ok(path);
        }
    }

    Err("llama-swap binary not found in PATH. Please install llama-swap first and ensure it's available in your PATH.".into())
}

fn generate_plist_content(binary_path: &str) -> crate::Result<String> {
    let log_path = expand_tilde(crate::constants::LOG_FILE_PATH)?;
    let working_dir = get_home_dir()?;

    let config_path = expand_tilde(crate::constants::CONFIG_FILE_PATH)?;

    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
        <string>-config</string>
        <string>{}</string>
        <string>-listen</string>
        <string>:{}</string>
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
        config_path,
        crate::constants::API_PORT,
        working_dir,
        log_path,
        log_path
    );

    Ok(plist)
}

fn create_default_config() -> &'static str {
    r#"# Llama-Swap Configuration
models:
  "Qwen3-30B-A3B-128K":
        cmd: >-
        llama-server
        --metrics
        --port 8902
        --model unsloth_Qwen3-30B-A3B-128K-GGUF_Qwen3-30B-A3B-128K-XXX.gguf
        --n-gpu-layers 999
        --flash-attn
        --rope-scaling yarn
        --rope-scale 4
        --yarn-orig-ctx 32768
        --ctx-size 131072
        --cache-type-k q4_1
        --cache-type-v q4_1
        --batch-size 1024
        --temp 0.6
        --top-p 0.95
        --top-k 20
        --min-p 0
"#
}
