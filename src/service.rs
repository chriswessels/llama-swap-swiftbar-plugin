use std::process::Command;
use crate::constants::LAUNCH_AGENT_LABEL;

#[allow(dead_code)]
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