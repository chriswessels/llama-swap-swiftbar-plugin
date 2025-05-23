use llama_swap_swiftbar::{PluginState, menu};
use llama_swap_swiftbar::models::{ServiceStatus, MetricsHistory, TimestampedValue};
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    println!("=== Menu Preview Tool ===\n");
    
    // Test 1: Running service with metrics
    println!("1. Running Service with Metrics:");
    println!("{}", "-".repeat(50));
    let state = create_mock_state_with_metrics();
    match menu::build_menu(&state) {
        Ok(menu_str) => {
            println!("{}", menu_str);
            // Save to file for SwiftBar testing
            std::fs::write("test_menu_running.txt", &menu_str).unwrap();
        }
        Err(e) => println!("Error building menu: {}", e),
    }
    
    // Test 2: Stopped service
    println!("\n2. Stopped Service:");
    println!("{}", "-".repeat(50));
    let mut stopped_state = create_mock_state_with_metrics();
    stopped_state.current_status = ServiceStatus::Stopped;
    match menu::build_menu(&stopped_state) {
        Ok(menu_str) => {
            println!("{}", menu_str);
            std::fs::write("test_menu_stopped.txt", &menu_str).unwrap();
        }
        Err(e) => println!("Error building menu: {}", e),
    }
    
    // Test 3: High memory warning
    println!("\n3. High Memory Warning:");
    println!("{}", "-".repeat(50));
    let mut high_mem_state = create_mock_state_with_metrics();
    // Add high memory value
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    high_mem_state.metrics_history.memory_mb.push_back(TimestampedValue {
        timestamp: now,
        value: 5120.0, // 5GB
    });
    match menu::build_menu(&high_mem_state) {
        Ok(menu_str) => {
            println!("{}", menu_str);
            std::fs::write("test_menu_high_memory.txt", &menu_str).unwrap();
        }
        Err(e) => println!("Error building menu: {}", e),
    }
    
    // Test 4: Error menu
    println!("\n4. Error Menu:");
    println!("{}", "-".repeat(50));
    let error_menu = menu::build_error_menu("Connection failed: timeout").unwrap();
    println!("{}", error_menu);
    std::fs::write("test_menu_error.txt", &error_menu).unwrap();
    
    // Test 5: Not installed menu
    println!("\n5. Not Installed Menu:");
    println!("{}", "-".repeat(50));
    let not_installed = menu::build_not_installed_menu();
    println!("{}", not_installed);
    std::fs::write("test_menu_not_installed.txt", &not_installed).unwrap();
    
    println!("\n✅ All test menus saved to txt files for SwiftBar testing");
}

fn create_mock_state_with_metrics() -> PluginState {
    let mut state = PluginState {
        current_status: ServiceStatus::Running,
        metrics_history: MetricsHistory::new(),
        error_count: 0,
    };
    
    // Add some mock metrics data
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    
    // Add model count data (0-3 models over time)
    for i in 0..20 {
        state.metrics_history.tps.push_back(TimestampedValue {
            timestamp: now - (20 - i) * 60,
            value: ((i as f64 / 5.0).sin() + 1.0) * 1.5,
        });
    }
    
    // Add memory data (varying between 1-2GB)
    for i in 0..20 {
        state.metrics_history.memory_mb.push_back(TimestampedValue {
            timestamp: now - (20 - i) * 60,
            value: 1024.0 + ((i as f64 / 3.0).cos() + 1.0) * 512.0,
        });
    }
    
    // Add cache hit rate data (60-90%)
    for i in 0..20 {
        state.metrics_history.cache_hit_rate.push_back(TimestampedValue {
            timestamp: now - (20 - i) * 60,
            value: 75.0 + ((i as f64 / 4.0).sin()) * 15.0,
        });
    }
    
    state
}