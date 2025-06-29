# Code Style and Conventions

## Rust Style Guidelines
- Follow Rust standard style (enforced by rustfmt)
- Use descriptive variable names
- Add doc comments for public APIs
- Keep functions focused and small
- Use type aliases for clarity (e.g., `pub type Result<T> = std::result::Result<T, Box<dyn Error>>`)

## Error Handling Pattern
```rust
// Use custom Result type with context helpers
pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

// Use error_helpers module for common patterns
use crate::types::error_helpers::{with_context, CONNECT_API};

pub fn fetch_metrics() -> Result<Metrics> {
    let response = client.get(url)
        .send()
        .map_err(|e| format!("{}: {}", CONNECT_API, e))?;
    
    let metrics: Metrics = with_context(response.json(), "Failed to parse JSON")?;
    
    Ok(metrics)
}

// Handle errors gracefully in UI
match fetch_metrics() {
    Ok(metrics) => display_metrics(metrics),
    Err(e) => {
        eprintln!("Metrics error: {}", e);
        display_offline_state()
    }
}
```

## State Management Pattern
```rust
// Clear separation of concerns
pub struct PluginState {
    // Core data
    pub http_client: Client,
    pub metrics_history: AllMetricsHistory,
    pub current_all_metrics: Option<AllMetrics>,
    
    // State tracking
    pub agent_state: AgentState,
    pub polling_mode: PollingMode,
    pub model_states: HashMap<String, ModelState>,
    pub service_status: ServiceStatus,
}

// State transitions are explicit and logged
impl PluginState {
    pub fn update_agent_state(&mut self) {
        let old_state = self.agent_state;
        let new_state = AgentState::from_system_check(/* ... */);
        
        if self.agent_state != old_state {
            eprintln!("Agent state: {old_state:?} -> {new_state:?}");
            self.last_state_change = Instant::now();
        }
        
        self.agent_state = new_state;
    }
}
```

## Module Organization
- Each module has a single responsibility
- Public API at the top of files
- Implementation details below
- Constants in dedicated constants.rs file
- Types in dedicated types.rs file
- State machine logic in state_model.rs

## Testing Conventions
- Tests in external `tests/` directory
- Group related tests in separate files
- Use descriptive test function names
- Test both success and failure cases
- Use helper functions to create test data

## Performance Considerations
- Minimize allocations in hot paths (streaming loop)
- Reuse System objects for sysinfo calls
- Preserve historical data across API failures
- Use CircularQueue for efficient bounded history
- Profile before optimizing
- Keep binary size under 5MB (aggressive release optimization)

## Naming Conventions
- Snake_case for functions and variables
- PascalCase for types, enums, and traits
- SCREAMING_SNAKE_CASE for constants
- Descriptive names over abbreviations
- Clear state machine naming (AgentState, DisplayState, PollingMode)