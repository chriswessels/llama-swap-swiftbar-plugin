# Suggested Commands for Development

## Build Commands
```bash
# Debug build
cargo build

# Release build (optimized for size)
cargo build --release

# Watch mode for auto-rebuild during development
cargo watch -x run
```

## Testing Commands
```bash
# Run all tests
cargo test

# Run specific test file
cargo test --test metrics_tests
cargo test --test install_ux_tests

# Run specific test function
cargo test test_collect_system_metrics_returns_valid_data

# Run with output
cargo test -- --nocapture

# Run benchmarks (if available)
cargo bench
```

## Code Quality Commands
```bash
# Format code
cargo fmt

# Check formatting without applying
cargo fmt -- --check

# Run linter
cargo clippy

# Run linter with all targets and features (enforce zero warnings)
cargo clippy --all-targets --all-features -- -D warnings
```

## Running the Plugin
```bash
# Run in debug mode with verbose output
LLAMA_SWAP_DEBUG=1 cargo run

# Test specific commands
cargo run -- do_start
cargo run -- do_stop
cargo run -- do_restart
cargo run -- view_logs
cargo run -- install_service
cargo run -- uninstall_service

# Run with specific environment
RUST_BACKTRACE=1 cargo run
```

## Documentation
```bash
# Generate and open documentation
cargo doc --open

# Generate docs with private items
cargo doc --document-private-items
```

## Utility Commands (macOS)
```bash
# Search for files
find . -name "*.rs"

# Search in files (use ripgrep)
rg "pattern" --type rust

# Git operations
git status
git diff
git add .
git commit -m "message"
```

## Release Process
```bash
# Strip symbols from release binary
strip target/release/llama-swap-swiftbar

# Check binary size
du -h target/release/llama-swap-swiftbar
```

## Development Testing
```bash
# Test install flow
cargo run -- install_service

# Test service management
launchctl list | grep llama-swap
launchctl print gui/$(id -u)/com.llama-swap.agent
```