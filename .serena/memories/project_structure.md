# Project Structure

## Directory Layout
```
llama-swap-swiftbar-plugin/
├── src/                    # Source code
│   ├── main.rs            # Entry point, streaming loop
│   ├── lib.rs             # Library root
│   ├── constants.rs       # Configuration constants
│   ├── models.rs          # Data structures
│   ├── menu.rs            # Menu construction
│   ├── metrics.rs         # API client
│   ├── charts.rs          # Sparkline rendering
│   ├── icons.rs           # Icon generation
│   ├── commands.rs        # Command handlers
│   └── service.rs         # Service management
├── assets/                # Static assets
│   └── llama-icon.png     # Base icon asset
├── docs/                  # Documentation
│   ├── README.md
│   ├── developer.md       # Developer guide
│   ├── installation.md
│   └── specs/            # Technical specifications
├── target/               # Build output (gitignored)
├── Cargo.toml            # Rust package manifest
├── Cargo.lock            # Dependency lock file
└── README.md             # Project readme
```

## Module Responsibilities
- **main.rs**: Application entry, command dispatch, streaming loop
- **models.rs**: Data types for API responses, state management
- **menu.rs**: SwiftBar menu generation and formatting
- **metrics.rs**: API client for fetching metrics
- **charts.rs**: Sparkline chart generation using image crate
- **icons.rs**: Dynamic icon generation with status indicators
- **commands.rs**: Handlers for menu commands (start/stop/etc)
- **service.rs**: LaunchAgent service management
- **constants.rs**: All configuration constants

## Key Design Patterns
- Streaming architecture for continuous updates
- Adaptive polling based on service state
- Graceful degradation for offline/error states
- History buffers for time-series charts