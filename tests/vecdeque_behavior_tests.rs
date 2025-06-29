use llama_swap_swiftbar::models::{DataAnalyzer, MetricsHistory};
use std::collections::VecDeque;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vecdeque_push_value_behavior() {
        let mut deque = VecDeque::new();
        let max_size = 3;

        // Test normal insertion
        DataAnalyzer::push_value_to_deque(&mut deque, 1.0, 100, max_size);
        DataAnalyzer::push_value_to_deque(&mut deque, 2.0, 200, max_size);
        DataAnalyzer::push_value_to_deque(&mut deque, 3.0, 300, max_size);

        assert_eq!(deque.len(), 3);
        assert_eq!(deque[0].value, 1.0);
        assert_eq!(deque[0].timestamp, 100);
        assert_eq!(deque[2].value, 3.0);
        assert_eq!(deque[2].timestamp, 300);

        // Test overflow behavior (should remove oldest)
        DataAnalyzer::push_value_to_deque(&mut deque, 4.0, 400, max_size);

        assert_eq!(deque.len(), 3);
        assert_eq!(deque[0].value, 2.0); // Oldest (1.0) should be removed
        assert_eq!(deque[0].timestamp, 200);
        assert_eq!(deque[2].value, 4.0);
        assert_eq!(deque[2].timestamp, 400);
    }

    #[test]
    fn test_vecdeque_stats_calculation() {
        let mut deque = VecDeque::new();
        let max_size = 5;

        // Add known values
        DataAnalyzer::push_value_to_deque(&mut deque, 1.0, 100, max_size);
        DataAnalyzer::push_value_to_deque(&mut deque, 2.0, 200, max_size);
        DataAnalyzer::push_value_to_deque(&mut deque, 3.0, 300, max_size);
        DataAnalyzer::push_value_to_deque(&mut deque, 4.0, 400, max_size);
        DataAnalyzer::push_value_to_deque(&mut deque, 5.0, 500, max_size);

        let stats = DataAnalyzer::get_stats(&deque);

        assert_eq!(stats.count, 5);
        assert_eq!(stats.current, 5.0);
        assert_eq!(stats.min, 1.0);
        assert_eq!(stats.max, 5.0);
        assert_eq!(stats.mean, 3.0);
        // Standard deviation of [1,2,3,4,5] should be sqrt(2) ≈ 1.414
        assert!((stats.std_dev - std::f64::consts::SQRT_2).abs() < 0.0001);
    }

    #[test]
    fn test_vecdeque_trim_functionality() {
        let mut deque = VecDeque::new();
        let max_size = 10;

        // Add values with different timestamps
        DataAnalyzer::push_value_to_deque(&mut deque, 1.0, 100, max_size);
        DataAnalyzer::push_value_to_deque(&mut deque, 2.0, 200, max_size);
        DataAnalyzer::push_value_to_deque(&mut deque, 3.0, 300, max_size);
        DataAnalyzer::push_value_to_deque(&mut deque, 4.0, 400, max_size);

        assert_eq!(deque.len(), 4);

        // Trim values older than 250
        DataAnalyzer::trim_deque(&mut deque, 250);

        assert_eq!(deque.len(), 2);
        assert_eq!(deque[0].timestamp, 300);
        assert_eq!(deque[1].timestamp, 400);
    }

    #[test]
    fn test_metrics_history_integration() {
        use llama_swap_swiftbar::models::Metrics;

        let mut history = MetricsHistory::with_capacity(3);

        let metrics1 = Metrics {
            predicted_tokens_per_sec: 10.0,
            prompt_tokens_per_sec: 5.0,
            memory_mb: 100.0,
            requests_processing: 1,
            requests_deferred: 2,
            n_decode_total: 0,
        };

        let metrics2 = Metrics {
            predicted_tokens_per_sec: 20.0,
            prompt_tokens_per_sec: 10.0,
            memory_mb: 200.0,
            requests_processing: 2,
            requests_deferred: 3,
            n_decode_total: 0,
        };

        let metrics3 = Metrics {
            predicted_tokens_per_sec: 30.0,
            prompt_tokens_per_sec: 15.0,
            memory_mb: 300.0,
            requests_processing: 3,
            requests_deferred: 4,
            n_decode_total: 0,
        };

        // Push metrics and verify behavior
        history.push(&metrics1);
        assert_eq!(history.tps.len(), 1);
        assert_eq!(history.queue_size.len(), 1);

        history.push(&metrics2);
        assert_eq!(history.tps.len(), 2);

        history.push(&metrics3);
        assert_eq!(history.tps.len(), 3);

        // Verify queue size calculation (processing + deferred)
        assert_eq!(history.queue_size.iter().next().unwrap().value, 7.0); // 3 + 4 (newest value)

        // Test overflow with capacity 3
        let metrics4 = Metrics {
            predicted_tokens_per_sec: 40.0,
            prompt_tokens_per_sec: 20.0,
            memory_mb: 400.0,
            requests_processing: 4,
            requests_deferred: 5,
            n_decode_total: 0,
        };

        history.push(&metrics4);
        assert_eq!(history.tps.len(), 3); // Should stay at capacity
        assert_eq!(history.tps.iter().last().unwrap().value, 20.0); // metrics1 should be gone (oldest in CircularQueue)
        assert_eq!(history.tps.iter().next().unwrap().value, 40.0); // metrics4 should be newest
    }

    #[test]
    fn test_empty_deque_stats() {
        let empty_deque = VecDeque::new();
        let stats = DataAnalyzer::get_stats(&empty_deque);

        assert_eq!(stats.count, 0);
        assert_eq!(stats.current, 0.0);
        assert_eq!(stats.mean, 0.0);
        assert_eq!(stats.min, 0.0);
        assert_eq!(stats.max, 0.0);
        assert_eq!(stats.std_dev, 0.0);
    }
}
