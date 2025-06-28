use crate::constants::LAUNCH_AGENT_LABEL;
use std::process::Command;

/// Check if service is loaded in launchctl (registered but may not be running)
pub fn is_service_loaded() -> bool {
    Command::new("launchctl")
        .args(["list", LAUNCH_AGENT_LABEL])
        .output()
        .ok()
        .map(|result| result.status.success())
        .unwrap_or(false)
}

/// Check if service is running via launchctl (has an active PID)
pub fn is_service_running() -> bool {
    Command::new("launchctl")
        .args(["list", LAUNCH_AGENT_LABEL])
        .output()
        .ok()
        .filter(|result| result.status.success())
        .and_then(|result| {
            let output_str = String::from_utf8_lossy(&result.stdout);

            // Check if the output contains a PID (indicating the process is actually running)
            // When a service is loaded but not running, launchctl returns a config dict without a PID
            // When a service is running, the output contains '"PID" = 12345;'
            if output_str.contains("\"PID\"") {
                // Extract the PID value and check if it's a valid number
                for line in output_str.lines() {
                    if line.trim().starts_with("\"PID\"") {
                        if let Some(pid_str) = line.split('=').nth(1) {
                            let pid_clean = pid_str
                                .trim()
                                .trim_end_matches(';')
                                .trim_matches('"')
                                .trim();
                            return Some(pid_clean.parse::<i32>().is_ok() && pid_clean != "0");
                        }
                    }
                }
            }
            Some(false)
        })
        .unwrap_or(false)
}
