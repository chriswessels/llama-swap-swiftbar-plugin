use reqwest::blocking::Client;
use crate::models::{Metrics, MetricsResponse, RunningResponse};
use crate::constants;
use std::time::Duration;

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
    
    // For now, estimate memory usage based on model count (rough estimate: 1GB per small model)
    let model_count = running_response.running.len();
    let estimated_memory_bytes: u64 = model_count as u64 * 1_073_741_824; // 1GB per model estimate
    
    // Create metrics response
    let metrics_response = MetricsResponse {
        running_models: running_response.running,
        total_memory_bytes: estimated_memory_bytes,
        model_count,
    };
    
    // Convert to internal metrics format
    let mut metrics: Metrics = metrics_response.into();
    metrics.validate()?;
    
    Ok(metrics)
}

/// Alternative: Check service status more explicitly
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
    fn test_metrics_parsing() {
        // Test JSON parsing with sample data
        let json = r#"{
            "running": [
                {
                    "model": "llama3.2:1b",
                    "state": "ready"
                }
            ]
        }"#;
        
        let running_response: RunningResponse = serde_json::from_str(json).unwrap();
        assert_eq!(running_response.running.len(), 1);
        
        let metrics_response = MetricsResponse {
            running_models: running_response.running,
            total_memory_bytes: 1073741824,
            model_count: 1,
        };
        
        let metrics: Metrics = metrics_response.into();
        
        assert_eq!(metrics.tps, 1.0); // model count
        assert_eq!(metrics.memory_mb, 1024.0); // 1GB in MB
        assert_eq!(metrics.cache_hit_rate, 0.0); // Not available
    }
}