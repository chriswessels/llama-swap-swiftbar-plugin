use std::process::Command;
use crate::constants::LAUNCH_AGENT_LABEL;

/// Check if service is running via launchctl
pub fn is_service_running() -> bool {
    Command::new("launchctl")
        .args(&["list", LAUNCH_AGENT_LABEL])
        .output()
        .ok()
        .filter(|result| result.status.success())
        .and_then(|result| {
            let output_str = String::from_utf8_lossy(&result.stdout);
            let parts: Vec<&str> = output_str.split_whitespace().collect();
            parts.first().map(|&pid| pid != "-")
        })
        .unwrap_or(false)
}