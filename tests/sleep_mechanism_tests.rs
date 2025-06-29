use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

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
    fn test_improved_adaptive_sleep_full_duration() {
        let (tx, rx) = mpsc::channel();
        let start = Instant::now();

        improved_adaptive_sleep(Duration::from_millis(500), &rx);

        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(450));
        assert!(elapsed <= Duration::from_millis(600));

        // Ensure tx is not dropped during test
        drop(tx);
    }

    #[test]
    fn test_improved_adaptive_sleep_early_termination() {
        let (tx, rx) = mpsc::channel();
        let start = Instant::now();

        // Start a thread that will send shutdown signal after 200ms
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(200));
            let _ = tx.send(());
        });

        improved_adaptive_sleep(Duration::from_millis(1000), &rx);

        let elapsed = start.elapsed();
        // Should terminate early, around 200ms
        assert!(elapsed >= Duration::from_millis(180));
        assert!(elapsed <= Duration::from_millis(300));
    }

    #[test]
    fn test_improved_adaptive_sleep_short_duration() {
        let (tx, rx) = mpsc::channel();
        let start = Instant::now();

        // Test with a short duration - should work correctly now
        improved_adaptive_sleep(Duration::from_millis(100), &rx);

        let elapsed = start.elapsed();
        // Should sleep for approximately the requested duration
        assert!(elapsed >= Duration::from_millis(90));
        assert!(elapsed <= Duration::from_millis(150));

        // Ensure tx is not dropped during test
        drop(tx);
    }

    #[test]
    fn test_improved_adaptive_sleep_zero_duration() {
        let (tx, rx) = mpsc::channel();
        let start = Instant::now();

        // Test with zero duration - should return immediately
        improved_adaptive_sleep(Duration::ZERO, &rx);

        let elapsed = start.elapsed();
        // Should return almost immediately
        assert!(elapsed <= Duration::from_millis(10));

        // Ensure tx is not dropped during test
        drop(tx);
    }
}
