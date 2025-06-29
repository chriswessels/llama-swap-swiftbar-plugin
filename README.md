# Llama-Swap SwiftBar Plugin

[![CI](https://github.com/your-org/llama-swap-swiftbar-plugin/actions/workflows/ci.yml/badge.svg)](https://github.com/your-org/llama-swap-swiftbar-plugin/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/your-org/llama-swap-swiftbar-plugin)](https://github.com/your-org/llama-swap-swiftbar-plugin/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A macOS menu bar plugin for SwiftBar that provides real-time monitoring and control of the Llama-Swap multi-model AI inference service. Monitor AI model performance, manage service lifecycle, and visualize metrics directly from your menu bar.

## What is Llama-Swap?

Llama-Swap is a multi-model AI inference service built on llama.cpp infrastructure that enables:
- **Concurrent AI Model Hosting**: Run multiple LLM instances simultaneously
- **Dynamic Model Management**: Load and unload models on demand
- **Request Queuing**: Intelligent queue management for inference requests
- **Performance Metrics**: Real-time monitoring via Prometheus-compatible metrics
- **Web Interface**: Browser-based model management UI

## Features

### 🔍 Real-time AI Monitoring
- **Inference Metrics**: Tokens per second (prompt + generation), memory usage per model
- **Queue Tracking**: Active requests, deferred requests, total processing count
- **Multi-Model Support**: Monitor multiple AI models running concurrently
- **System Resources**: CPU usage, memory consumption, process monitoring
- **Historical Data**: 5-minute metrics retention with automatic cleanup

### 📊 Visual Analytics
- **Sparkline Charts**: Time-series visualization directly in menu bar
- **Dynamic Status Icons**: Color-coded indicators showing service state
- **Performance Trends**: Track TPS, memory, and queue processing over time
- **Adaptive Display**: Context-aware information density based on activity

### ⚙️ Service Management
- **LaunchAgent Control**: Full macOS service lifecycle management
- **Automatic Installation**: Seamless service setup with guided installation
- **Process Monitoring**: Multi-layer health detection (plist, launchctl, process, API)
- **Error Recovery**: Graceful handling of service failures with state preservation
- **Web UI Integration**: Direct access to Llama-Swap management interface

### 🧠 Intelligent Operation
- **Adaptive Polling**: Frequency adjusts automatically based on activity levels
- **State Machine**: Sophisticated tracking of agent, service, and model states
- **Smart Sleep**: Optimized resource usage during idle periods
- **Context Preservation**: Historical metrics survive service restarts and failures

## Installation

### Prerequisites
- **macOS 10.15+** (Catalina or later)
- **SwiftBar** installed ([download here](https://swiftbar.app/))

### Option 1: Shell Script Wrapper (Recommended)

This approach uses a shell script wrapper that provides better integration with SwiftBar annotations and easy environment variable configuration.

1. **Install SwiftBar**
   ```bash
   # Via Homebrew
   brew install --cask swiftbar
   ```

2. **Download Binary and Script**
   ```bash
   # Create directory for binaries
   mkdir -p ~/.local/bin
   
   # Download binary for your architecture
   # For Apple Silicon (M1/M2/M3/M4)
   curl -L -o ~/.local/bin/llama-swap-swiftbar \
     https://github.com/your-org/llama-swap-swiftbar-plugin/releases/latest/download/llama-swap-swiftbar-arm64
   
   # For Intel Macs
   curl -L -o ~/.local/bin/llama-swap-swiftbar \
     https://github.com/your-org/llama-swap-swiftbar-plugin/releases/latest/download/llama-swap-swiftbar-x64
   
   # Make binary executable
   chmod +x ~/.local/bin/llama-swap-swiftbar
   
   # Download shell script wrapper
   curl -L -o ~/Library/Application\ Support/SwiftBar/llama-swap-swiftbar.sh \
     https://github.com/your-org/llama-swap-swiftbar-plugin/releases/latest/download/llama-swap-swiftbar.sh
   
   # Make script executable
   chmod +x ~/Library/Application\ Support/SwiftBar/llama-swap-swiftbar.sh
   ```

3. **Customize Configuration (Optional)**
   ```bash
   # Edit the wrapper script to customize environment variables
   nano ~/Library/Application\ Support/SwiftBar/llama-swap-swiftbar.sh
   ```

4. **Refresh SwiftBar** and the plugin should appear in your menu bar

For users who prefer a simpler approach without wrapper scripts:

```bash
# Install SwiftBar
brew install --cask swiftbar

# Download and install binary directly
# For Apple Silicon
curl -L -o ~/Library/Application\ Support/SwiftBar/llama-swap-swiftbar \
  https://github.com/your-org/llama-swap-swiftbar-plugin/releases/latest/download/llama-swap-swiftbar-arm64

# For Intel
curl -L -o ~/Library/Application\ Support/SwiftBar/llama-swap-swiftbar \
  https://github.com/your-org/llama-swap-swiftbar-plugin/releases/latest/download/llama-swap-swiftbar-x64

# Make executable and refresh SwiftBar
chmod +x ~/Library/Application\ Support/SwiftBar/llama-swap-swiftbar
```

### Option 3: Development/Build from Source

**Additional Prerequisites:**
- **Rust toolchain** ([install via rustup](https://rustup.rs/))

**Build Steps:**

1. **Install SwiftBar**
   ```bash
   # Via Homebrew
   brew install --cask swiftbar
   
   # Or download from https://swiftbar.app/
   ```

2. **Clone and Build**
   ```bash
   git clone https://github.com/your-org/llama-swap-swiftbar-plugin.git
   cd llama-swap-swiftbar-plugin
   cargo build --release
   ```

3. **Install Plugin**
   ```bash
   # Copy to SwiftBar plugins directory
   cp target/release/llama-swap-swiftbar ~/Library/Application\ Support/SwiftBar/
   
   # Make executable
   chmod +x ~/Library/Application\ Support/SwiftBar/llama-swap-swiftbar
   ```

4. **Activate Plugin**
   - Refresh SwiftBar or restart it
   - The Llama-Swap icon should appear in your menu bar

### Service Installation
The plugin will automatically detect if Llama-Swap service is not installed and provide installation options through the menu interface.

## Usage

### Menu Bar Interface
- **Status Icon**: Color-coded indicator showing current service state
  - 🔴 Red: Service stopped or missing requirements
  - 🟡 Yellow: Service starting or model loading
  - 🟢 Green: Models ready and idle
  - 🔵 Blue: Processing requests
  - ⚪ Grey: Service running but no models loaded

- **Click Menu**: Opens detailed metrics with charts and controls
- **Sparklines**: Visual representation of performance trends

### Service Controls
- **Start Service**: Launch Llama-Swap daemon via LaunchAgent
- **Stop Service**: Gracefully shutdown service
- **Restart Service**: Full service restart cycle
- **Unload Models**: Free memory by unloading all AI models
- **Install Service**: Automatic LaunchAgent setup and configuration
- **Uninstall Service**: Clean removal of service components

### File Management
- **View Logs**: Open service logs in default text editor
- **Edit Configuration**: Access Llama-Swap configuration file
- **Open Web UI**: Launch browser to Llama-Swap management interface

### Monitoring Information
- **System Stats**: CPU usage, memory consumption, system health
- **Model Metrics**: Per-model performance including:
  - Prompt processing speed (tokens/sec)
  - Generation speed (tokens/sec)
  - Memory usage (MB)
  - Queue status (active/deferred requests)
- **Historical Charts**: 5-minute rolling history with statistical analysis

## Architecture

### Technology Stack
- **Core Language**: Rust 2021 Edition
- **Menu Integration**: bitbar crate with image processing support
- **HTTP Client**: reqwest with blocking mode and rustls-tls
- **Data Processing**: serde/serde_json for API communication
- **Image Generation**: image + png crates for charts and icons
- **System Monitoring**: sysinfo for resource tracking
- **Data Storage**: circular-queue for efficient metrics history
- **Process Management**: ctrlc for graceful shutdown handling

### API Integration
The plugin communicates with Llama-Swap via REST API:
- **`GET /running`**: List active models and their states
- **`GET /upstream/{model}/metrics`**: Prometheus metrics per model
- **`GET /unload`**: Unload all models to free memory
- **Web UI**: Available at `http://127.0.0.1:45786/ui/models`

### State Management
- **AgentState**: Service installation and lifecycle tracking
- **DisplayState**: UI presentation logic based on current conditions
- **PollingMode**: Adaptive update frequency (1s active, 3s idle)
- **ModelState**: Individual AI model status tracking

## Development

### Continuous Integration

The project includes GitHub Actions CI that:
- ✅ Runs tests on every push and PR
- ✅ Enforces code formatting and linting (zero warnings)
- ✅ Builds binaries for both Apple Silicon (ARM64) and Intel (x64)
- ✅ Creates universal binaries for releases
- ✅ Automatically uploads release artifacts

CI Status: ![CI](https://github.com/your-org/llama-swap-swiftbar-plugin/actions/workflows/ci.yml/badge.svg)

### Building
```bash
# Debug build with full symbols
cargo build

# Optimized release build (under 5MB)
cargo build --release

# Cross-compile for different architectures
cargo build --release --target aarch64-apple-darwin  # Apple Silicon
cargo build --release --target x86_64-apple-darwin   # Intel

# Development with auto-rebuild
cargo watch -x run

# Create universal binary (requires both targets built)
lipo -create -output llama-swap-swiftbar-universal \
  target/aarch64-apple-darwin/release/llama-swap-swiftbar \
  target/x86_64-apple-darwin/release/llama-swap-swiftbar
```

### Testing
```bash
# Run all test suites
cargo test

# Specific test categories
cargo test --test metrics_tests        # System metrics validation
cargo test --test install_ux_tests     # Installation user experience
cargo test --test sleep_mechanism_tests # Polling and sleep behavior

# Verbose test output
cargo test -- --nocapture
```

### Code Quality
```bash
# Format code to Rust standards
cargo fmt

# Lint with zero-warning policy
cargo clippy --all-targets --all-features -- -D warnings
```

### Running & Debugging
```bash
# Debug mode with verbose logging
LLAMA_SWAP_DEBUG=1 cargo run

# Test specific plugin commands
cargo run -- do_start      # Start service
cargo run -- do_stop       # Stop service
cargo run -- install_service  # Install LaunchAgent
cargo run -- open_ui       # Open web interface

# Environment configuration
RUST_BACKTRACE=1 cargo run  # Show stack traces
LLAMA_SWAP_API_PORT=8080 cargo run  # Custom API port
```

## Configuration

### Shell Script Wrapper Configuration

The recommended installation uses a shell script wrapper that makes configuration easy. Edit your wrapper script to set environment variables:

```bash
# Edit the wrapper script
nano ~/Library/Application\ Support/SwiftBar/llama-swap-swiftbar.sh
```

**Common Configuration Examples:**

```bash
# Remote Llama-Swap instance
export LLAMA_SWAP_API_BASE_URL="http://192.168.1.100"
export LLAMA_SWAP_API_PORT="8080"
export LLAMA_SWAP_API_TIMEOUT_SECS="3"

# Larger charts for better visibility
export LLAMA_SWAP_CHART_WIDTH="80"
export LLAMA_SWAP_CHART_HEIGHT="30"

# Extended history (10 minutes)
export LLAMA_SWAP_HISTORY_SIZE="600"

# Debug mode for troubleshooting
export LLAMA_SWAP_DEBUG="true"
```

**Development Setup:**
```bash
# Use the development wrapper script
cp scripts/llama-swap-swiftbar-dev.sh ~/Library/Application\ Support/SwiftBar/
# Update PROJECT_PATH in the script to point to your clone
```

### Environment Variables Reference

| Variable | Default | Description |
|----------|---------|-------------|
| `LLAMA_SWAP_API_BASE_URL` | `http://127.0.0.1` | Base URL for Llama-Swap API |
| `LLAMA_SWAP_API_PORT` | `45786` | API port number |
| `LLAMA_SWAP_API_TIMEOUT_SECS` | `1` | Request timeout in seconds |
| `LLAMA_SWAP_STREAMING_MODE` | `true` | Enable continuous streaming updates |
| `LLAMA_SWAP_CHART_WIDTH` | `60` | Sparkline chart width in pixels |
| `LLAMA_SWAP_CHART_HEIGHT` | `20` | Sparkline chart height in pixels |
| `LLAMA_SWAP_HISTORY_SIZE` | `300` | Number of metric samples to retain (5 min @ 1s) |
| `LLAMA_SWAP_DEBUG` | `false` | Enable verbose debug logging |
| `LLAMA_SWAP_LOG_FILE_PATH` | `~/Library/Logs/LlamaSwap.log` | Custom log file location |
| `LLAMA_SWAP_CONFIG_FILE_PATH` | `~/.llamaswap/config.yaml` | Custom config file location |

### SwiftBar Annotations

The shell script wrapper includes these SwiftBar annotations for optimal integration:

```bash
#<swiftbar.type>streamable</swiftbar.type>                    # Enable streaming updates
#<swiftbar.hideAbout>true</swiftbar.hideAbout>                # Hide About menu
#<swiftbar.hideRunInTerminal>true</swiftbar.hideRunInTerminal># Hide terminal option
#<swiftbar.hideLastUpdated>true</swiftbar.hideLastUpdated>    # Hide update timestamp
#<swiftbar.hideDisablePlugin>true</swiftbar.hideDisablePlugin># Hide disable option
#<swiftbar.hideSwiftBar>true</swiftbar.hideSwiftBar>          # Hide SwiftBar submenu
```

### File Locations
- **Service Logs**: `~/Library/Logs/LlamaSwap.log`
- **Configuration**: `~/.llamaswap/config.yaml`
- **LaunchAgent**: `~/Library/LaunchAgents/com.user.llama-swap.plist`

### Customization
Key settings can be modified in `src/constants.rs`:
- API endpoints and timeouts
- Chart colors and dimensions
- Polling intervals and history retention
- LaunchAgent configuration
- Status colors for different states

## Troubleshooting

### Common Issues

1. **Plugin not visible in SwiftBar**
   - Verify SwiftBar is running and menu bar icons are enabled
   - Check file permissions: `chmod +x ~/Library/Application\ Support/SwiftBar/llama-swap-swiftbar`
   - Review SwiftBar console for plugin loading errors

2. **Service fails to start**
   - Ensure llama-swap binary is available in PATH
   - Check user has write permissions to `~/Library/LaunchAgents/`
   - Verify LaunchAgent plist syntax: `plutil ~/Library/LaunchAgents/com.user.llama-swap.plist`

3. **API connection errors**
   - Confirm Llama-Swap service is running: `launchctl list | grep llama-swap`
   - Test API connectivity: `curl http://127.0.0.1:45786/running`
   - Check firewall settings and port availability

4. **Metrics not updating**
   - Enable debug mode: `LLAMA_SWAP_DEBUG=1` for detailed logs
   - Verify service process is active and responsive
   - Check for network connectivity issues

### Debug Mode
Enable comprehensive logging for troubleshooting:
```bash
# Set environment variable for debug output
export LLAMA_SWAP_DEBUG=1

# Or run directly with debug logging
LLAMA_SWAP_DEBUG=1 /path/to/llama-swap-swiftbar
```

## Contributing

### Development Workflow
1. Fork the repository and clone locally
2. Install Rust toolchain via [rustup](https://rustup.rs/)
3. Run test suite: `cargo test`
4. Format code: `cargo fmt`
5. Lint code: `cargo clippy`
6. Submit pull request with tests

### Code Standards
- **Rust Conventions**: Follow standard rustfmt formatting
- **Zero Warnings**: All clippy warnings must be resolved
- **Test Coverage**: New features require corresponding tests
- **Documentation**: Public APIs must have doc comments
- **Performance**: Profile changes that affect the streaming loop

### Testing Strategy
- **Unit Tests**: Function-level validation in `tests/` directory
- **Integration Tests**: Full user experience flows
- **System Tests**: Service management and LaunchAgent operations
- **Error Handling**: Failure recovery and graceful degradation

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- **[SwiftBar](https://swiftbar.app/)** for the excellent macOS menu bar framework
- **[llama.cpp](https://github.com/ggerganov/llama.cpp)** for the underlying AI inference engine
- **Rust Community** for outstanding development tools and ecosystem
- **Prometheus** for standardized metrics collection patterns