# Developer Guide

## Development Setup

### Prerequisites

- Rust 1.70 or later
- macOS 12.0+ (for testing)
- SwiftBar (for live testing)
- Python 3 (for mock service)

### Initial Setup

1. **Clone and Setup**
   ```bash
   git clone https://github.com/your-org/llama-swap-swiftbar.git
   cd llama-swap-swiftbar
   
   # Install development tools
   cargo install cargo-watch
   cargo install cargo-expand
   rustup component add clippy rustfmt
   ```

2. **Install Dependencies**
   ```bash
   cargo build
   ```

## Project Structure

```
llama-swap-swiftbar/
├── src/
│   ├── main.rs          # Entry point, streaming loop
│   ├── constants.rs     # Configuration constants
│   ├── models.rs        # Data structures
│   ├── menu.rs          # Menu construction
│   ├── metrics.rs       # API client
│   ├── charts.rs        # Sparkline rendering
│   ├── icons.rs         # Icon generation
│   ├── commands.rs      # Command handlers
│   └── persistence.rs   # Data persistence
├── assets/
│   └── llama-icon.png   # Base icon asset
├── tests/               # Integration tests
├── benches/             # Performance benchmarks
└── tools/               # Development utilities
```

## Development Workflow

### Running Locally

1. **Start Mock Service**
   ```bash
   python3 tools/mock_service.py
   ```

2. **Run Plugin in Terminal**
   ```bash
   # Debug mode with verbose output
   LLAMA_SWAP_DEBUG=1 cargo run
   
   # Watch mode for auto-rebuild
   cargo watch -x run
   ```

3. **Test Specific Commands**
   ```bash
   # Test command handling
   cargo run -- do_start
   cargo run -- view_logs
   ```

### Testing with SwiftBar

1. **Build and Install**
   ```bash
   ./tools/dev_install.sh
   ```

2. **View Real-time Logs**
   ```bash
   # In one terminal
   tail -f /tmp/llama-swap-plugin.log
   
   # In another terminal
   tail -f ~/Library/Logs/LlamaSwap.log
   ```

## Code Guidelines

### Style Guidelines

- Follow Rust standard style (enforced by rustfmt)
- Use descriptive variable names
- Add doc comments for public APIs
- Keep functions focused and small

### Error Handling

```rust
// Use Result type for fallible operations
pub fn fetch_metrics() -> Result<Metrics> {
    // Provide context for errors
    let response = client.get(url)
        .send()
        .map_err(|e| format!("Failed to connect: {}", e))?;
    
    // Validate data
    let mut metrics: Metrics = response.json()?;
    metrics.validate()?;
    
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

### Performance Considerations

- Minimize allocations in hot paths
- Reuse buffers where possible
- Profile before optimizing
- Keep binary size under 5MB

## Testing

### Unit Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_menu_generation

# Run with output
cargo test -- --nocapture
```

### Integration Tests

```bash
# Run integration tests
cargo test --test integration_test

# Test with mock service
./tools/run_integration_tests.sh
```

### Benchmarks

```bash
# Run performance benchmarks
cargo bench

# Generate flame graph
cargo flamegraph --bench performance
```

### Manual Testing Checklist

- [ ] Service detection (running/stopped)
- [ ] Start/stop commands work
- [ ] Metrics display correctly
- [ ] Charts render properly
- [ ] File operations (logs/config)
- [ ] Error states handled
- [ ] Memory usage stable
- [ ] CPU usage minimal

## Adding Features

### Adding a New Metric

1. **Update Models** (src/models.rs)
   ```rust
   pub struct MetricsResponse {
       // ... existing fields
       pub new_metric: f64,
   }
   ```

2. **Update History** (src/models.rs)
   ```rust
   pub struct MetricsHistory {
       // ... existing fields
       pub new_metric: VecDeque<TimestampedValue>,
   }
   ```

3. **Add Chart Generation** (src/charts.rs)
   ```rust
   pub fn generate_new_metric_sparkline(history: &VecDeque<f64>) -> Result<DynamicImage> {
       generate_sparkline(history, COLOR_NEW_METRIC, CHART_WIDTH, CHART_HEIGHT)
   }
   ```

4. **Update Menu** (src/menu.rs)
   ```rust
   if let Some(item) = self.create_metric_item(
       "New Metric",
       &history.new_metric,
       charts::generate_new_metric_sparkline,
       |v| format!("{:.1}", v),
       false,
   ) {
       items.push(item);
   }
   ```

### Adding a New Command

1. **Add Command Handler** (src/commands.rs)
   ```rust
   #[bitbar::command]
   fn do_new_action() -> Result<()> {
       // Implementation
       Ok(())
   }
   ```

2. **Update Command Dispatch**
   ```rust
   pub fn handle_command(command: &str) -> Result<()> {
       match command {
           // ... existing commands
           "do_new_action" => do_new_action(),
           _ => Err(format!("Unknown command: {}", command).into()),
       }
   }
   ```

3. **Add Menu Item** (src/menu.rs)
   ```rust
   items.push(
       MenuItem::new("New Action")
           .command(Command::exec("do_new_action"))
   );
   ```

## Debugging

### Debug Output

```rust
// Use debug macro
debug_log!("Processing metrics: {:?}", metrics);

// Conditional compilation for debug
#[cfg(debug_assertions)]
eprintln!("Debug: Current state = {:?}", state);
```

### Common Issues

**Plugin Crashes**
- Check panic messages in Console.app
- Enable RUST_BACKTRACE=1
- Add defensive error handling

**Performance Issues**
- Profile with cargo flamegraph
- Check for unnecessary allocations
- Verify sleep intervals

**Visual Glitches**
- Test in both light/dark mode
- Verify image dimensions
- Check base64 encoding

## Release Process

1. **Update Version**
   ```toml
   # Cargo.toml
   version = "0.2.0"
   ```

2. **Run Release Checklist**
   ```bash
   ./tools/pre_release_check.sh
   ```

3. **Build Release**
   ```bash
   cargo build --release
   strip target/release/llama-swap-swiftbar
   ```

4. **Create Release**
   ```bash
   git tag v0.2.0
   git push origin v0.2.0
   ```

## Resources

- [SwiftBar Documentation](https://github.com/swiftbar/SwiftBar)
- [BitBar Plugin API](https://github.com/matryer/bitbar#plugin-api)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Image Crate Docs](https://docs.rs/image/)

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Run `cargo fmt` and `cargo clippy`
6. Submit a pull request

See [CONTRIBUTING.md](../CONTRIBUTING.md) for detailed guidelines.