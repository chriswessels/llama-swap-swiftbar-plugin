use reqwest::blocking::Client;
use crate::models::{Metrics, RunningResponse, RunningModel, AllMetrics, ModelMetrics, SystemMetrics};
use crate::constants;
use crate::types::error_helpers::{with_context, CONNECT_API, PARSE_JSON};
use std::time::Duration;
use std::collections::HashMap;

#[derive(Debug)]
struct PrometheusMetric {
    name: String,
    value: f64,
    #[allow(dead_code)]
    labels: HashMap<String, String>,
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
        let labels_str = &metric_part[label_start+1..metric_part.len()-1];
        let mut labels = HashMap::new();
        
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

fn parse_prometheus_metrics(text: &str) -> HashMap<String, f64> {
    const METRIC_MAPPINGS: &[(&str, &str)] = &[
        ("llamacpp:prompt_tokens_seconds", "prompt_tokens_per_sec"),
        ("llamacpp:predicted_tokens_seconds", "predicted_tokens_per_sec"),
        ("llamacpp:requests_processing", "requests_processing"),
        ("llamacpp:requests_deferred", "requests_deferred"),
        ("llamacpp:kv_cache_usage_ratio", "kv_cache_usage_ratio"),
        ("llamacpp:kv_cache_tokens", "kv_cache_tokens"),
        ("llamacpp:n_decode_total", "n_decode_total"),
    ];

    text.lines()
        .filter_map(parse_prometheus_line)
        .filter_map(|metric| {
            METRIC_MAPPINGS
                .iter()
                .find(|(source, _)| *source == metric.name)
                .map(|(_, target)| (target.to_string(), metric.value))
        })
        .collect()
}

fn collect_system_metrics() -> SystemMetrics {
    use sysinfo::System;
    
    let mut system = System::new_all();
    system.refresh_all();
    
    // CPU usage
    system.refresh_cpu_all();
    std::thread::sleep(Duration::from_millis(200));
    system.refresh_cpu_all();
    
    let cpu_usage_percent = system.global_cpu_usage() as f64;
    
    // Memory metrics
    let total_memory_bytes = system.total_memory();
    let used_memory_bytes = system.used_memory();
    let available_memory_bytes = system.available_memory();
    
    let total_memory_gb = bytes_to_gb(total_memory_bytes);
    let used_memory_gb = bytes_to_gb(used_memory_bytes);
    let available_memory_gb = bytes_to_gb(available_memory_bytes);
    let memory_usage_percent = percentage(used_memory_bytes, total_memory_bytes);
        
    SystemMetrics {
        cpu_usage_percent,
        total_memory_gb,
        used_memory_gb,
        available_memory_gb,
        memory_usage_percent,
    }
}

fn get_llama_server_memory_mb() -> f64 {
    use sysinfo::System;
    
    let system = System::new_all();
    let total_memory_kb = system
        .processes()
        .values()
        .filter(|process| {
            let name = process.name().to_string_lossy();
            name.contains("llama-server") || name.contains("llama_server") || name.contains("llama-swap")
        })
        .map(|process| process.memory())
        .sum::<u64>();
    
    total_memory_kb as f64 / 1024.0
}

fn fetch_model_metrics(client: &Client, model: &RunningModel) -> HashMap<String, f64> {
    let url = format!(
        "{}:{}/upstream/{}/metrics", 
        constants::API_BASE_URL, 
        constants::API_PORT,
        model.model.replace(":", "%3A")
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

fn create_metrics_from_data(data: &HashMap<String, f64>) -> crate::Result<Metrics> {
    let metrics = Metrics {
        prompt_tokens_per_sec: get_metric_value(data, "prompt_tokens_per_sec"),
        predicted_tokens_per_sec: get_metric_value(data, "predicted_tokens_per_sec"),
        requests_processing: get_metric_value(data, "requests_processing") as u32,
        requests_deferred: get_metric_value(data, "requests_deferred") as u32,
        kv_cache_usage_ratio: get_metric_value(data, "kv_cache_usage_ratio"),
        kv_cache_tokens: get_metric_value(data, "kv_cache_tokens") as u32,
        n_decode_total: get_metric_value(data, "n_decode_total") as u32,
        memory_mb: 0.0,
    };
    
    Ok(metrics)
}

pub fn fetch_all_metrics(client: &Client) -> crate::Result<AllMetrics> {
    let url = format!("{}:{}/running", constants::API_BASE_URL, constants::API_PORT);
    
    let response = with_context(
        client.get(&url).send(),
        CONNECT_API
    )?;
    
    if !response.status().is_success() {
        return Err(format!("API returned error: {}", response.status()).into());
    }
    
    let running_response: RunningResponse = with_context(
        response.json(),
        PARSE_JSON
    )?;
    
    let llama_memory_mb = get_llama_server_memory_mb();
    let system_metrics = collect_system_metrics();
    
    let models = running_response
        .running
        .iter()
        .map(|model| {
            let model_state = model.model_state();
            let metrics = if model_state == crate::models::ModelState::Running {
                let model_metrics_data = fetch_model_metrics(client, model);
                create_metrics_from_data(&model_metrics_data).unwrap_or_default()
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
    
    Ok(AllMetrics {
        models,
        total_llama_memory_mb: llama_memory_mb,
        system_metrics,
    })
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
    
    #[test]
    fn test_helper_functions() {
        assert_eq!(bytes_to_gb(1_073_741_824), 1.0);
        assert_eq!(percentage(50, 100), 50.0);
        assert_eq!(percentage(0, 100), 0.0);
        assert_eq!(percentage(100, 0), 0.0);
    }
}