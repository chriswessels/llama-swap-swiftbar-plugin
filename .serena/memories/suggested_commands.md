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

# Run specific test
cargo test test_menu_generation

# Run with output
cargo test -- --nocapture

# Run benchmarks
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

# Run linter with all targets
cargo clippy --all-targets --all-features
```

## Running the Plugin
```bash
# Run in debug mode with verbose output
LLAMA_SWAP_DEBUG=1 cargo run

# Test specific commands
cargo run -- do_start
cargo run -- view_logs

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
# List files
ls -la

# Change directory
cd <path>

# Search for files
find . -name "*.rs"

# Search in files (use ripgrep on macOS)
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