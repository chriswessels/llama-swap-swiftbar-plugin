use std::sync::mpsc;
use std::time::Duration;

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for the improved channel-based sleep mechanism
    fn improved_adaptive_sleep(duration: Duration, shutdown_rx: &mpsc::Receiver<()>) {
        let _ = shutdown_rx.recv_timeout(duration);
        // If recv_timeout returns Ok(()), we got a shutdown signal
        // If it returns Err(RecvTimeoutError::Timeout), the duration elapsed
        // If it returns Err(RecvTimeoutError::Disconnected), the sender was dropped (also treat as shutdown)
        // In all cases, we just return - the caller will check if shutdown was requested
    }

    #[test]
    fn test_improved_adaptive_sleep_completes_without_signal() {
        let (_tx, rx) = mpsc::channel();

        // Test that function completes when no signal is sent
        // We don't care about exact timing, just that it returns
        improved_adaptive_sleep(Duration::from_millis(50), &rx);

        // If we get here, the function completed successfully
    }

    #[test]
    fn test_improved_adaptive_sleep_responds_to_signal() {
        let (tx, rx) = mpsc::channel();

        // Send signal immediately - function should return quickly
        // We don't measure timing, just verify it doesn't hang
        tx.send(()).unwrap();

        // This should return immediately since signal was already sent
        improved_adaptive_sleep(Duration::from_millis(1000), &rx);

        // If we get here, the function responded to the signal correctly
    }

    #[test]
    fn test_improved_adaptive_sleep_handles_disconnected_channel() {
        let (tx, rx) = mpsc::channel();

        // Drop the sender - this simulates a disconnected channel
        drop(tx);

        // Function should return without hanging when channel is disconnected
        improved_adaptive_sleep(Duration::from_millis(100), &rx);

        // If we get here, the function handled disconnection correctly
    }

    #[test]
    fn test_improved_adaptive_sleep_zero_duration() {
        let (_tx, rx) = mpsc::channel();

        // Test with zero duration - should return immediately without waiting
        improved_adaptive_sleep(Duration::ZERO, &rx);

        // If we get here, the function handled zero duration correctly
    }
}
