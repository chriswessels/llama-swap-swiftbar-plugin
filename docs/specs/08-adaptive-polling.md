# Phase 8: Adaptive Polling Architecture Specification

## Overview

This phase transforms the current fixed 5-second polling loop into an adaptive system that adjusts polling frequency based on service activity. When the service is actively processing requests, the plugin polls more frequently to provide real-time feedback. When idle, it reduces polling to conserve resources.

## Goals

- Implement adaptive polling frequency based on queue processing activity
- Maintain responsive updates during active inference workloads
- Reduce resource usage when service is idle
- Preserve SwiftBar streaming compatibility
- Keep implementation simple and maintainable

## Current State Analysis

### Existing Polling System
- Fixed 5-second interval (`constants::UPDATE_INTERVAL_SECS`)
- Synchronous metrics fetching in `update_state()`
- Simple timer-based loop in `run_streaming_mode()`

### Pain Points
- 5-second delay to see service start/stop events
- Same delay during active inference when users want real-time updates
- Unnecessary frequent polling when service is idle
- No differentiation between active vs idle states

## Adaptive Polling Strategy

### Polling Intervals
```rust
pub enum PollingMode {
    Idle,       // 5 seconds - no requests processing
    Active,     // 1 second - requests being processed
    Starting,   // 2 seconds - service transitioning states
}

impl PollingMode {
    pub fn interval_secs(&self) -> u64 {
        match self {
            PollingMode::Idle => 5,
            PollingMode::Active => 1,
            PollingMode::Starting => 2,
        }
    }
}
```

### State Transition Logic
```
┌─────────┐    queue > 0    ┌────────┐
│   Idle  │ ──────────────→ │ Active │
│ (5 sec) │                 │ (1 sec)│
└─────────┘ ←────────────── └────────┘
      ↑       queue = 0          ↓
      │                          │
      │     ┌──────────┐         │
      └──── │ Starting │ ←───────┘
            │ (2 sec)  │ service state change
            └──────────┘
```

## Implementation

### 8.1 Enhanced State Management

Update `src/main.rs` with adaptive polling state:

```rust
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PollingMode {
    Idle,       // Service running but no queue activity
    Active,     // Requests being processed
    Starting,   // Service state transitions
}

impl PollingMode {
    pub fn interval_secs(&self) -> u64 {
        match self {
            PollingMode::Idle => 5,
            PollingMode::Active => 1, 
            PollingMode::Starting => 2,
        }
    }
    
    pub fn description(&self) -> &'static str {
        match self {
            PollingMode::Idle => "idle polling",
            PollingMode::Active => "active polling", 
            PollingMode::Starting => "transition polling",
        }
    }
}

pub struct PluginState {
    pub http_client: Client,
    pub metrics_history: MetricsHistory,
    pub current_status: ServiceStatus,
    pub current_metrics: Option<models::Metrics>,
    pub error_count: usize,
    pub polling_mode: PollingMode,
    pub last_status: ServiceStatus,
    pub mode_change_time: Instant,
}

impl PluginState {
    fn new() -> Result<Self> {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(constants::API_TIMEOUT_SECS))
            .build()?;
        
        let metrics_history = MetricsHistory::new();

        Ok(Self {
            http_client,
            metrics_history,
            current_status: ServiceStatus::Unknown,
            current_metrics: None,
            error_count: 0,
            polling_mode: PollingMode::Starting, // Start with faster polling
            last_status: ServiceStatus::Unknown,
            mode_change_time: Instant::now(),
        })
    }
    
    /// Update polling mode based on current metrics and status
    fn update_polling_mode(&mut self) {
        let new_mode = self.determine_polling_mode();
        
        if new_mode != self.polling_mode {
            eprintln!("Polling mode: {} -> {} ({})", 
                self.polling_mode.description(),
                new_mode.description(),
                self.get_mode_reason()
            );
            
            self.polling_mode = new_mode;
            self.mode_change_time = Instant::now();
        }
    }
    
    fn determine_polling_mode(&self) -> PollingMode {
        // Handle service state transitions
        if self.current_status != self.last_status {
            return PollingMode::Starting;
        }
        
        // Stay in Starting mode for at least 10 seconds after state change
        if self.polling_mode == PollingMode::Starting && 
           self.mode_change_time.elapsed() < Duration::from_secs(10) {
            return PollingMode::Starting;
        }
        
        // Check if service is actively processing requests
        if let Some(ref metrics) = self.current_metrics {
            if metrics.requests_processing > 0 || metrics.requests_deferred > 0 {
                return PollingMode::Active;
            }
        }
        
        // Default to idle when service is running but no queue activity
        if self.current_status == ServiceStatus::Running {
            PollingMode::Idle
        } else {
            PollingMode::Starting
        }
    }
    
    fn get_mode_reason(&self) -> String {
        if self.current_status != self.last_status {
            return format!("status changed: {:?} -> {:?}", self.last_status, self.current_status);
        }
        
        if let Some(ref metrics) = self.current_metrics {
            if metrics.requests_processing > 0 {
                return format!("processing {} requests", metrics.requests_processing);
            }
            if metrics.requests_deferred > 0 {
                return format!("{} requests queued", metrics.requests_deferred);
            }
        }
        
        "no queue activity".to_string()
    }
}
```

### 8.2 Adaptive Main Loop

Update the streaming mode function:

```rust
fn run_streaming_mode() -> Result<()> {
    // Set up shutdown flag
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    
    // Handle Ctrl+C and termination signals
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })?;
    
    // Initialize state and output
    let mut state = PluginState::new()?;
    
    eprintln!("Starting adaptive polling mode");
    
    // Main loop with adaptive timing
    while running.load(Ordering::SeqCst) {
        let loop_start = Instant::now();
        
        // Render current frame
        let frame = render_frame(&mut state)?;
        
        print!("~~~\n{}", frame);
        io::stdout().flush()?;
        
        // Determine sleep duration based on current mode
        let sleep_duration = Duration::from_secs(state.polling_mode.interval_secs());
        
        // Adaptive interruptible sleep
        adaptive_sleep(sleep_duration, &running);
        
        // Optional: Log timing for debugging
        if cfg!(debug_assertions) {
            let loop_duration = loop_start.elapsed();
            if loop_duration > Duration::from_millis(500) {
                eprintln!("Slow loop iteration: {:?} (mode: {})", 
                    loop_duration, state.polling_mode.description());
            }
        }
    }
    
    eprintln!("Plugin shutting down gracefully");
    Ok(())
}

/// Interruptible sleep that respects shutdown signal
fn adaptive_sleep(duration: Duration, running: &Arc<AtomicBool>) {
    let sleep_chunks = duration.as_secs().max(1); // At least 1 second chunks
    let chunk_duration = Duration::from_secs(1);
    
    for _ in 0..sleep_chunks {
        if !running.load(Ordering::SeqCst) {
            break;
        }
        thread::sleep(chunk_duration);
    }
    
    // Handle sub-second remainder
    let remainder = duration - Duration::from_secs(sleep_chunks);
    if remainder > Duration::ZERO && running.load(Ordering::SeqCst) {
        thread::sleep(remainder);
    }
}
```

### 8.3 Enhanced State Update

Update `update_state` to track status changes:

```rust
fn update_state(state: &mut PluginState) {
    // Store previous status for comparison
    state.last_status = state.current_status;
    
    // Primary check: try to fetch metrics
    match metrics::fetch_metrics(&state.http_client) {
        Ok(metrics) => {
            // Service is running and responsive
            state.current_status = ServiceStatus::Running;
            state.metrics_history.push(&metrics);
            state.current_metrics = Some(metrics);
            state.error_count = 0; // Reset error count on success
        }
        Err(e) => {
            eprintln!("Metrics fetch failed: {}", e);
            state.error_count += 1;
            
            // Secondary check: is service actually running?
            if service::is_service_running(service::DetectionMethod::LaunchctlList) {
                // Service is running but API is not responsive
                state.current_status = ServiceStatus::Running;
                eprintln!("Service is running but API is not responding");
            } else {
                // Service is truly stopped
                state.current_status = ServiceStatus::Stopped;
            }
            
            // Clear metrics when service is not responsive
            state.current_metrics = None;
        }
    }
    
    // Update polling mode based on new state
    state.update_polling_mode();
}
```

### 8.4 Enhanced Constants

Update `src/constants.rs` for adaptive polling:

```rust
// Update timing constants
pub const UPDATE_INTERVAL_SECS: u64 = 5;     // Default/fallback interval
pub const ACTIVE_INTERVAL_SECS: u64 = 1;     // When processing requests
pub const STARTING_INTERVAL_SECS: u64 = 2;   // During state transitions

// Adaptive polling configuration
pub const MIN_STARTING_DURATION_SECS: u64 = 10;  // Minimum time in Starting mode
pub const STREAMING_MODE: bool = true;

// Polling mode thresholds
pub const QUEUE_ACTIVE_THRESHOLD: u32 = 1;    // Switch to active if queue > 0
```

### 8.5 Menu Integration

Enhance menu display to show polling status (optional debug info):

```rust
// In src/menu.rs, add to footer section:

fn add_footer_section(&mut self, state: &crate::PluginState) {
    // Existing version info...
    
    // Add polling mode indicator in debug builds
    if cfg!(debug_assertions) {
        let mode_text = format!("Polling: {} ({}s)", 
            state.polling_mode.description(),
            state.polling_mode.interval_secs()
        );
        
        let mut mode_item = ContentItem::new(mode_text);
        mode_item = mode_item.color("#666666").unwrap();
        mode_item = mode_item.font("Menlo").size(10);
        self.items.push(MenuItem::Content(mode_item));
    }
}
```

## Benefits of Adaptive Polling

### Responsiveness Improvements
- **Service state changes**: Detected within 1-2 seconds instead of up to 5 seconds
- **Active inference**: Real-time updates during request processing
- **Queue status**: Immediate visibility into request processing flow

### Resource Efficiency
- **Idle service**: 80% reduction in API calls (5s vs 1s intervals)
- **Network usage**: Proportional reduction in HTTP requests
- **CPU usage**: Lower background CPU when service is idle

### User Experience
- **Active workloads**: Live updates show generation progress
- **Service management**: Faster feedback on start/stop operations
- **Queue monitoring**: Real-time visibility into processing status

## Implementation Phases

### Phase 1: Basic Adaptive Logic
1. Add `PollingMode` enum and state tracking
2. Implement `determine_polling_mode()` logic
3. Update main loop with adaptive sleep
4. Test mode transitions

### Phase 2: Queue-Based Adaptation
1. Add queue processing thresholds
2. Implement Starting mode timeout
3. Add debug logging for mode changes
4. Validate against real workloads

### Phase 3: Fine-Tuning
1. Optimize interval values based on testing
2. Add hysteresis to prevent mode thrashing
3. Performance monitoring and metrics
4. Documentation and user guidelines

## Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_polling_mode_transitions() {
        let mut state = PluginState::new().unwrap();
        
        // Initially should be Starting mode
        assert_eq!(state.polling_mode, PollingMode::Starting);
        
        // Set metrics with active queue
        state.current_metrics = Some(Metrics {
            requests_processing: 2,
            requests_deferred: 1,
            ..Default::default()
        });
        state.current_status = ServiceStatus::Running;
        
        state.update_polling_mode();
        assert_eq!(state.polling_mode, PollingMode::Active);
        
        // Clear queue
        state.current_metrics = Some(Metrics {
            requests_processing: 0,
            requests_deferred: 0,
            ..Default::default()
        });
        
        state.update_polling_mode();
        assert_eq!(state.polling_mode, PollingMode::Idle);
    }
    
    #[test]
    fn test_starting_mode_timeout() {
        let mut state = PluginState::new().unwrap();
        state.polling_mode = PollingMode::Starting;
        state.mode_change_time = Instant::now() - Duration::from_secs(15);
        
        // Should transition out of Starting mode after timeout
        state.current_status = ServiceStatus::Running;
        state.update_polling_mode();
        
        assert_ne!(state.polling_mode, PollingMode::Starting);
    }
}
```

### Integration Testing
1. **Service lifecycle**: Start service, verify Starting → Idle transition
2. **Queue activity**: Submit requests, verify Idle → Active transition
3. **Mixed workloads**: Alternating active/idle periods
4. **Error conditions**: API failures, service crashes, network issues

### Performance Validation
```bash
# Monitor polling frequency in real scenarios
tail -f ~/Library/Logs/LlamaSwap.log | grep "Polling mode"

# Resource usage comparison
time ./target/release/llama-swap-swiftbar --benchmark-mode

# Network traffic analysis
sudo tcpdump -i lo0 port 45786
```

## Migration Strategy

### Backward Compatibility
- Maintain existing `UPDATE_INTERVAL_SECS` as fallback
- Keep same SwiftBar streaming output format
- Preserve all existing command-line interfaces

### Configuration Options
```rust
// Optional: Add configuration for power users
pub struct AdaptiveConfig {
    pub idle_interval_secs: u64,
    pub active_interval_secs: u64,
    pub starting_interval_secs: u64,
    pub starting_timeout_secs: u64,
    pub enabled: bool,
}

impl Default for AdaptiveConfig {
    fn default() -> Self {
        Self {
            idle_interval_secs: 5,
            active_interval_secs: 1,
            starting_interval_secs: 2,
            starting_timeout_secs: 10,
            enabled: true,
        }
    }
}
```

### Rollback Plan
If adaptive polling causes issues:
1. Add feature flag: `ADAPTIVE_POLLING=false`
2. Fall back to original fixed 5-second interval
3. Maintain all existing functionality

## Performance Expectations

### Typical Scenarios
- **Idle service**: API calls every 5 seconds (same as current)
- **Active inference**: API calls every 1 second (5x increase during activity)
- **Service restart**: API calls every 2 seconds for 10 seconds, then adaptive

### Resource Impact
- **Baseline usage**: No change when service is idle
- **Active usage**: Temporary increase during inference (typically 30s-2min bursts)
- **Network**: Proportional to activity level, not constant overhead

## Future Enhancements

### Smart Prediction
- Learn typical inference patterns
- Pre-emptively switch to Active mode based on time-of-day
- Adaptive timeout based on historical queue duration

### Performance Metrics
- Track polling mode distribution over time
- Measure response time improvements
- Monitor resource usage patterns

### Advanced Adaptation
- GPU utilization as additional trigger
- Model loading events as state transitions
- Integration with system-wide activity monitoring

## Conclusion

This adaptive polling architecture provides a significant improvement in responsiveness while maintaining resource efficiency. The implementation is simple, maintainable, and fully backward compatible with the existing system.

Key benefits:
- **5x faster updates** during active inference workloads
- **Same resource usage** when service is idle  
- **Immediate feedback** on service state changes
- **Simple implementation** that builds on existing code

The system intelligently adapts to usage patterns, providing the best of both worlds: real-time responsiveness when needed, and efficient resource usage when idle.