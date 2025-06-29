use crate::constants;
use crate::models::{
    AllMetrics, Metrics, ModelMetrics, RunningModel, RunningResponse, SystemMetrics,
};
use crate::types::error_helpers::{with_context, CONNECT_API, PARSE_JSON};
use reqwest::blocking::Client;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub memory_mb: f64,

    pub inferred_model: Option<String>,
}

#[derive(Debug)]
struct PrometheusMetric {
    name: String,
    value: f64,
}

fn parse_prometheus_line(line: &str) -> Option<PrometheusMetric> {
    if line.starts_with('#') || line.trim().is_empty() {
        return None;
    }

    let parts: Vec<&str> = line.splitn(2, ' ').collect();
    if parts.len() != 2 {
        return None;
    }

    let metric_part = parts[0];
    let value = parts[1].parse::<f64>().ok()?;

    if let Some(label_start) = metric_part.find('{') {
        let name = metric_part[..label_start].to_string();

        Some(PrometheusMetric { name, value })
    } else {
        Some(PrometheusMetric {
            name: metric_part.to_string(),
            value,
        })
    }
}

fn parse_prometheus_metrics(text: &str) -> HashMap<String, f64> {
    const METRIC_MAPPINGS: &[(&str, &str)] = &[
        ("llamacpp:prompt_tokens_seconds", "prompt_tokens_per_sec"),
        (
            "llamacpp:predicted_tokens_seconds",
            "predicted_tokens_per_sec",
        ),
        ("llamacpp:requests_processing", "requests_processing"),
        ("llamacpp:requests_deferred", "requests_deferred"),
        ("llamacpp:n_decode_total", "n_decode_total"),
    ];

    let parsed_metrics: Vec<_> = text.lines().filter_map(parse_prometheus_line).collect();

    parsed_metrics
        .into_iter()
        .filter_map(|metric| {
            METRIC_MAPPINGS
                .iter()
                .find(|(source, _)| *source == metric.name)
                .map(|(_, target)| ((*target).to_string(), metric.value))
        })
        .collect()
}

pub fn collect_system_metrics(system: &mut sysinfo::System) -> SystemMetrics {
    system.refresh_all();

    // CPU usage
    system.refresh_cpu_all();
    std::thread::sleep(Duration::from_millis(200));
    system.refresh_cpu_all();

    let cpu_usage_percent = f64::from(system.global_cpu_usage());

    // Memory metrics
    let total_memory_bytes = system.total_memory();
    let used_memory_bytes = system.used_memory();

    let used_memory_gb = bytes_to_gb(used_memory_bytes);
    let memory_usage_percent = percentage(used_memory_bytes, total_memory_bytes);

    SystemMetrics {
        cpu_usage_percent,
        used_memory_gb,
        memory_usage_percent,
    }
}

pub fn get_llama_server_memory_mb(system: &sysinfo::System) -> f64 {
    get_detailed_llama_processes(system)
        .iter()
        .map(|p| p.memory_mb)
        .sum()
}

pub fn get_detailed_llama_processes(system: &sysinfo::System) -> Vec<ProcessInfo> {
    system
        .processes()
        .values()
        .filter_map(|process| {
            let name = process.name().to_string_lossy().to_string();
            let cmd_line = process
                .cmd()
                .iter()
                .map(|s| s.to_string_lossy())
                .collect::<Vec<_>>()
                .join(" ");

            // Only match actual llama binaries, not processes that mention them in paths
            let name_matches = name == "llama-server"
                || name == "llama-swap"
                || name == "llama-swap-swiftbar"
                || name == "llama-cli";
            let cmd_starts_with_llama = cmd_line.starts_with("llama-server")
                || cmd_line.starts_with("llama-swap")
                || cmd_line.starts_with("llama-cli")
                || cmd_line.contains("/llama-server ")
                || cmd_line.contains("/llama-swap ")
                || cmd_line.contains("/llama-cli ")
                || cmd_line.ends_with("llama-swap-swiftbar");

            if name_matches || cmd_starts_with_llama {
                let memory_mb = process.memory() as f64 / (1024.0 * 1024.0);
                let inferred_model = infer_model_from_command(&cmd_line);

                Some(ProcessInfo {
                    pid: process.pid().as_u32(),
                    name,
                    memory_mb,

                    inferred_model,
                })
            } else {
                None
            }
        })
        .collect()
}

fn infer_model_from_command(cmd_line: &str) -> Option<String> {
    // Extract model name from --model argument
    if let Some(model_start) = cmd_line.find("--model ") {
        let model_part = &cmd_line[model_start + 8..];
        if let Some(model_end) = model_part.find(' ') {
            let model_path = &model_part[..model_end];
            // Extract just the model name from the path
            if let Some(filename) = model_path.split('/').next_back() {
                // Remove .gguf extension if present
                let model_name = filename.strip_suffix(".gguf").unwrap_or(filename);
                return Some(model_name.to_string());
            }
        } else {
            // Model is the last argument
            if let Some(filename) = model_part.split('/').next_back() {
                let model_name = filename.strip_suffix(".gguf").unwrap_or(filename);
                return Some(model_name.to_string());
            }
        }
    }

    // Extract port for identification
    if let Some(port_start) = cmd_line.find("--port ") {
        let port_part = &cmd_line[port_start + 7..];
        if let Some(port_end) = port_part.find(' ') {
            let port = &port_part[..port_end];
            return Some(format!("Port {port}"));
        } else if !port_part.is_empty() {
            return Some(format!("Port {}", port_part.trim()));
        }
    }

    None
}

fn fetch_model_metrics(client: &Client, model: &RunningModel) -> HashMap<String, f64> {
    let url = format!(
        "{}:{}/upstream/{}/metrics",
        constants::API_BASE_URL,
        constants::API_PORT,
        model.model.replace(':', "%3A")
    );

    client
        .get(&url)
        .timeout(Duration::from_secs(1))
        .send()
        .ok()
        .filter(|response| response.status().is_success())
        .and_then(|response| response.text().ok())
        .map(|text| parse_prometheus_metrics(&text))
        .unwrap_or_default()
}

fn create_metrics_from_data(data: &HashMap<String, f64>) -> Metrics {
    Metrics {
        prompt_tokens_per_sec: get_metric_value(data, "prompt_tokens_per_sec"),
        predicted_tokens_per_sec: get_metric_value(data, "predicted_tokens_per_sec"),
        requests_processing: get_metric_value(data, "requests_processing") as u32,
        requests_deferred: get_metric_value(data, "requests_deferred") as u32,
        n_decode_total: get_metric_value(data, "n_decode_total") as u32,
        memory_mb: 0.0,
    }
}

pub fn fetch_all_metrics(client: &Client) -> crate::Result<AllMetrics> {
    let url = format!(
        "{}:{}/running",
        constants::API_BASE_URL,
        constants::API_PORT
    );

    let response = with_context(client.get(&url).send(), CONNECT_API)?;

    if !response.status().is_success() {
        return Err(format!("API returned error: {}", response.status()).into());
    }

    let running_response: RunningResponse = with_context(response.json(), PARSE_JSON)?;

    let models = running_response
        .running
        .iter()
        .map(|model| {
            let model_state = model.model_state();
            let metrics = if model_state == crate::models::ModelState::Running {
                let model_metrics_data = fetch_model_metrics(client, model);
                create_metrics_from_data(&model_metrics_data)
            } else {
                // For loading/unknown models, use empty metrics
                Metrics::default()
            };

            ModelMetrics {
                model_name: model.model.clone(),
                model_state,
                metrics,
            }
        })
        .collect();

    Ok(AllMetrics { models })
}

// Helper functions
fn bytes_to_gb(bytes: u64) -> f64 {
    bytes as f64 / 1_073_741_824.0
}

fn percentage(value: u64, total: u64) -> f64 {
    if total > 0 {
        (value as f64 / total as f64) * 100.0
    } else {
        0.0
    }
}

fn get_metric_value(data: &HashMap<String, f64>, key: &str) -> f64 {
    *data.get(key).unwrap_or(&0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prometheus_parsing() {
        let sample_prometheus = r"# HELP llamacpp:prompt_tokens_seconds Prompt tokens per second
# TYPE llamacpp:prompt_tokens_seconds gauge
llamacpp:prompt_tokens_seconds 150.5
# HELP llamacpp:predicted_tokens_seconds Predicted tokens per second  
# TYPE llamacpp:predicted_tokens_seconds gauge
llamacpp:predicted_tokens_seconds 25.3
# HELP llamacpp:requests_processing Number of requests being processed
# TYPE llamacpp:requests_processing gauge
llamacpp:requests_processing 2";

        let metrics = parse_prometheus_metrics(sample_prometheus);

        assert_eq!(metrics.get("prompt_tokens_per_sec"), Some(&150.5));
        assert_eq!(metrics.get("predicted_tokens_per_sec"), Some(&25.3));
        assert_eq!(metrics.get("requests_processing"), Some(&2.0));
    }

    #[test]
    fn test_prometheus_with_labels() {
        let sample = r#"llamacpp:prompt_tokens_seconds{model="llama3.2:1b"} 150.5"#;

        let metric = parse_prometheus_line(sample).unwrap();
        assert_eq!(metric.name, "llamacpp:prompt_tokens_seconds");
        assert_eq!(metric.value, 150.5);
    }

    #[test]
    fn test_helper_functions() {
        assert_eq!(bytes_to_gb(1_073_741_824), 1.0);
        assert_eq!(percentage(50, 100), 50.0);
        assert_eq!(percentage(0, 100), 0.0);
        assert_eq!(percentage(100, 0), 0.0);
    }
}
