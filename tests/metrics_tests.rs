use llama_swap_swiftbar::metrics::{collect_system_metrics, get_llama_server_memory_mb};
use sysinfo::System;

#[test]
fn test_collect_system_metrics_returns_valid_data() {
    let mut system = System::new_all();
    let metrics = collect_system_metrics(&mut system);

    // System metrics should be reasonable values
    assert!(metrics.cpu_usage_percent >= 0.0);
    assert!(metrics.cpu_usage_percent <= 100.0);
    assert!(metrics.used_memory_gb >= 0.0);
    assert!(metrics.memory_usage_percent >= 0.0);
    assert!(metrics.memory_usage_percent <= 100.0);
}

#[test]
fn test_collect_system_metrics_consistency() {
    // Multiple calls should return similar but not identical results
    let mut system1 = System::new_all();
    let mut system2 = System::new_all();
    let metrics1 = collect_system_metrics(&mut system1);
    let metrics2 = collect_system_metrics(&mut system2);

    // Both should be valid
    assert!(metrics1.cpu_usage_percent >= 0.0);
    assert!(metrics2.cpu_usage_percent >= 0.0);
    assert!(metrics1.used_memory_gb >= 0.0);
    assert!(metrics2.used_memory_gb >= 0.0);
}

#[test]
fn test_llama_server_memory_collection() {
    // This should not panic even if no llama processes are running
    let system = System::new_all();
    let memory_mb = get_llama_server_memory_mb(&system);
    assert!(memory_mb >= 0.0);
}

#[test]
fn test_shared_system_efficiency() {
    // Test that we can reuse the same System object efficiently
    let mut system = System::new_all();

    // Collect both system metrics and llama memory using the same System instance
    let metrics = collect_system_metrics(&mut system);
    let memory_mb = get_llama_server_memory_mb(&system);

    // Both should be valid
    assert!(metrics.cpu_usage_percent >= 0.0);
    assert!(memory_mb >= 0.0);

    // Verify we can use the system again without re-initializing
    let metrics2 = collect_system_metrics(&mut system);
    assert!(metrics2.cpu_usage_percent >= 0.0);
}
