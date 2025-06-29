# Project Structure

## Directory Layout
```
llama-swap-swiftbar-plugin/
├── src/                        # Source code
│   ├── main.rs                # Entry point, streaming loop, command dispatch
│   ├── lib.rs                 # Library root with module declarations
│   ├── types.rs               # Core types: PluginState, ServiceStatus, Result, error_helpers
│   ├── state_model.rs         # State machine: AgentState, DisplayState, PollingMode, ModelState
│   ├── models.rs              # Data models: API responses, metrics, history management
│   ├── menu.rs                # SwiftBar menu generation and formatting
│   ├── metrics.rs             # API client, system metrics collection
│   ├── charts.rs              # Sparkline chart generation using image crate  
│   ├── icons.rs               # Dynamic icon generation with status indicators
│   ├── commands.rs            # Command handlers: start/stop/install/service management
│   ├── service.rs             # LaunchAgent service status checking
│   └── constants.rs           # All configuration constants and colors
├── tests/                     # Test suite (external to src/)
│   ├── metrics_tests.rs       # System metrics collection validation
│   ├── install_ux_tests.rs    # Installation user experience tests
│   └── sleep_mechanism_tests.rs # Sleep/polling mechanism tests
├── assets/                    # Static assets
│   └── llama-icon.png         # Base icon asset
├── target/                    # Build output (gitignored)
├── .serena/                   # Serena project configuration
├── Cargo.toml                 # Rust package manifest
├── Cargo.lock                 # Dependency lock file
└── README.md                  # Project readme
```

## Key Module Responsibilities
- **main.rs**: Application entry, streaming loop, panic/signal handling
- **types.rs**: Core data types, plugin state, service status, error helpers
- **state_model.rs**: State machine definitions and transitions
- **models.rs**: API data models, metrics history with CircularQueue
- **menu.rs**: SwiftBar menu generation, formatting, command definitions
- **metrics.rs**: API client, system metrics, process monitoring
- **charts.rs**: Sparkline generation for time-series visualization
- **icons.rs**: Dynamic icon creation with status indicators
- **commands.rs**: Service management, LaunchAgent operations
- **service.rs**: Service status checking utilities
- **constants.rs**: Configuration, colors, timeouts

## Key Design Patterns
- **State Machine**: Clear separation of agent, display, and polling states
- **Streaming Architecture**: Continuous updates with adaptive polling
- **Service Layering**: Multi-layer service status (plist, launchctl, process, API)
- **Error Preservation**: Historical metrics preserved across failures
- **Test Coverage**: External test directory with focused test suites