use reqwest::blocking::Client;
use crate::models::{Metrics, RunningResponse, RunningModel};
use crate::constants;
use std::time::Duration;
use std::collections::HashMap;

/// Prometheus metric structure
#[derive(Debug)]
struct PrometheusMetric {
    name: String,
    value: f64,
    labels: HashMap<String, String>,
}

/// Parse a single Prometheus metric line
fn parse_prometheus_line(line: &str) -> Option<PrometheusMetric> {
    // Skip comments and empty lines
    if line.starts_with('#') || line.trim().is_empty() {
        return None;
    }
    
    // Split metric name and value
    let parts: Vec<&str> = line.splitn(2, ' ').collect();
    if parts.len() != 2 {
        return None;
    }
    
    let metric_part = parts[0];
    let value_str = parts[1];
    
    // Parse value
    let value = value_str.parse::<f64>().ok()?;
    
    // Parse metric name and labels
    if let Some(label_start) = metric_part.find('{') {
        let name = metric_part[..label_start].to_string();
        let labels_str = &metric_part[label_start+1..metric_part.len()-1];
        let mut labels = HashMap::new();
        
        // Simple label parsing
        for label_pair in labels_str.split(',') {
            if let Some(eq_pos) = label_pair.find('=') {
                let key = label_pair[..eq_pos].trim().to_string();
                let val = label_pair[eq_pos+1..].trim().trim_matches('"').to_string();
                labels.insert(key, val);
            }
        }
        
        Some(PrometheusMetric { name, value, labels })
    } else {
        Some(PrometheusMetric {
            name: metric_part.to_string(),
            value,
            labels: HashMap::new(),
        })
    }
}

/// Parse Prometheus metrics text format
fn parse_prometheus_metrics(text: &str) -> HashMap<String, f64> {
    let mut metrics = HashMap::new();
    
    for line in text.lines() {
        if let Some(metric) = parse_prometheus_line(line) {
            // We're interested in these specific metrics
            match metric.name.as_str() {
                "llamacpp:prompt_tokens_seconds" => {
                    metrics.insert("prompt_tokens_per_sec".to_string(), metric.value);
                }
                "llamacpp:predicted_tokens_seconds" => {
                    metrics.insert("predicted_tokens_per_sec".to_string(), metric.value);
                }
                "llamacpp:requests_processing" => {
                    metrics.insert("requests_processing".to_string(), metric.value);
                }
                _ => {}
            }
        }
    }
    
    metrics
}

/// Get memory usage by running ps command
fn get_llama_server_memory_mb() -> f64 {
    use std::process::Command;
    
    // Use ps to find llama-server processes
    let output = Command::new("ps")
        .args(&["aux"])
        .output();
    
    if let Ok(output) = output {
        if let Ok(stdout) = String::from_utf8(output.stdout) {
            let mut total_memory_kb = 0u64;
            
            for line in stdout.lines() {
                // Check if line contains llama-server
                if line.contains("llama-server") || line.contains("llama_server") {
                    // Parse memory from ps output (6th column is RSS in KB)
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() > 5 {
                        if let Ok(mem_kb) = parts[5].parse::<u64>() {
                            total_memory_kb += mem_kb;
                        }
                    }
                }
            }
            
            // Convert KB to MB
            return total_memory_kb as f64 / 1024.0;
        }
    }
    
    // Fallback to sysinfo if ps fails
    use sysinfo::System;
    let system = System::new_all();
    
    let mut total_memory_kb = 0u64;
    for (_, process) in system.processes() {
        let name = process.name().to_string_lossy();
        if name.contains("llama-server") || name.contains("llama_server") {
            total_memory_kb += process.memory();
        }
    }
    
    total_memory_kb as f64 / 1024.0
}

/// Fetch metrics for a specific model
fn fetch_model_metrics(client: &Client, model: &RunningModel) -> HashMap<String, f64> {
    let mut metrics = HashMap::new();
    
    // Construct the metrics URL for this model
    let url = format!("{}:{}/upstream/{}/metrics", 
        constants::API_BASE_URL, 
        constants::API_PORT,
        model.model.replace(":", "%3A") // URL encode the colon
    );
    
    // Try to fetch Prometheus metrics
    match client.get(&url).timeout(Duration::from_secs(1)).send() {
        Ok(response) if response.status().is_success() => {
            if let Ok(text) = response.text() {
                metrics = parse_prometheus_metrics(&text);
            }
        }
        _ => {}
    }
    
    metrics
}

/// Fetch metrics from the Llama-Swap API
pub fn fetch_metrics(client: &Client) -> crate::Result<Metrics> {
    let url = format!("{}:{}/running", constants::API_BASE_URL, constants::API_PORT);
    
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
    let running_response: RunningResponse = response
        .json()
        .map_err(|e| format!("Failed to parse running models JSON: {}", e))?;
    
    // Get memory usage from sysinfo
    let memory_mb = get_llama_server_memory_mb();
    
    // Aggregate metrics from all running models
    let mut total_prompt_tokens_per_sec = 0.0;
    let mut total_predicted_tokens_per_sec = 0.0;
    let mut total_requests_processing = 0u32;
    
    for model in &running_response.running {
        if model.state == "ready" {
            let model_metrics = fetch_model_metrics(client, model);
            
            total_prompt_tokens_per_sec += model_metrics.get("prompt_tokens_per_sec").unwrap_or(&0.0);
            total_predicted_tokens_per_sec += model_metrics.get("predicted_tokens_per_sec").unwrap_or(&0.0);
            total_requests_processing += *model_metrics.get("requests_processing").unwrap_or(&0.0) as u32;
        }
    }
    
    // Create metrics
    let mut metrics = Metrics {
        prompt_tokens_per_sec: total_prompt_tokens_per_sec,
        predicted_tokens_per_sec: total_predicted_tokens_per_sec,
        requests_processing: total_requests_processing,
        memory_mb,
    };
    
    metrics.validate()?;
    
    Ok(metrics)
}

/// Alternative: Check service health more explicitly
pub fn check_service_health(client: &Client) -> bool {
    let url = format!("{}:{}/running", constants::API_BASE_URL, constants::API_PORT);
    
    match client.get(&url).timeout(Duration::from_secs(1)).send() {
        Ok(response) => response.status().is_success(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_prometheus_parsing() {
        let sample_prometheus = r#"# HELP llamacpp:prompt_tokens_seconds Prompt tokens per second
# TYPE llamacpp:prompt_tokens_seconds gauge
llamacpp:prompt_tokens_seconds 150.5
# HELP llamacpp:predicted_tokens_seconds Predicted tokens per second  
# TYPE llamacpp:predicted_tokens_seconds gauge
llamacpp:predicted_tokens_seconds 25.3
# HELP llamacpp:requests_processing Number of requests being processed
# TYPE llamacpp:requests_processing gauge
llamacpp:requests_processing 2"#;
        
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
        assert_eq!(metric.labels.get("model"), Some(&"llama3.2:1b".to_string()));
    }
}