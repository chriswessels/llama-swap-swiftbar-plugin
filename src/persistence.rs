use std::fs;
use std::path::{Path, PathBuf};
use crate::models::MetricsHistory;

const PERSISTENCE_FILE: &str = "llama-swap-metrics.json";

/// Get the path for persistence file
fn get_persistence_path() -> crate::Result<PathBuf> {
    let home = std::env::var("HOME")
        .map_err(|_| "Failed to get HOME directory")?;
    
    let data_dir = Path::new(&home)
        .join("Library")
        .join("Application Support")
        .join("SwiftBar")
        .join("PluginData");
    
    // Create directory if it doesn't exist
    fs::create_dir_all(&data_dir)
        .map_err(|e| format!("Failed to create data directory: {}", e))?;
    
    Ok(data_dir.join(PERSISTENCE_FILE))
}

/// Save metrics history to disk
pub fn save_metrics(history: &MetricsHistory) -> crate::Result<()> {
    let path = get_persistence_path()?;
    
    let json = serde_json::to_string_pretty(history)
        .map_err(|e| format!("Failed to serialize metrics: {}", e))?;
    
    fs::write(&path, json)
        .map_err(|e| format!("Failed to write metrics file: {}", e))?;
    
    Ok(())
}

/// Load metrics history from disk
pub fn load_metrics() -> crate::Result<MetricsHistory> {
    let path = get_persistence_path()?;
    
    if !path.exists() {
        // No saved data, return empty history
        return Ok(MetricsHistory::new());
    }
    
    let json = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read metrics file: {}", e))?;
    
    let mut history: MetricsHistory = serde_json::from_str(&json)
        .map_err(|e| format!("Failed to parse metrics file: {}", e))?;
    
    // Set max_size since it's not serialized
    history.max_size = crate::constants::HISTORY_SIZE;
    
    // Trim any old data
    history.trim_old_data();
    
    Ok(history)
}

/// Delete persistence file
pub fn clear_persistence() -> crate::Result<()> {
    let path = get_persistence_path()?;
    
    if path.exists() {
        fs::remove_file(&path)
            .map_err(|e| format!("Failed to delete metrics file: {}", e))?;
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Metrics;
    
    #[test]
    fn test_save_load_cycle() {
        let mut history = MetricsHistory::new();
        
        // Add some test data
        let metrics = Metrics {
            prompt_tokens_per_sec: 524.0,
            predicted_tokens_per_sec: 42.0,
            requests_processing: 1,
            memory_mb: 1024.0,
        };
        
        history.push(&metrics);
        
        // Save
        assert!(save_metrics(&history).is_ok());
        
        // Load
        let loaded = load_metrics().unwrap();
        assert_eq!(loaded.tps.len(), 1);
        assert_eq!(loaded.tps[0].value, 42.0); // generation speed from predicted_tokens_per_sec
        
        // Cleanup
        let _ = clear_persistence();
    }
}