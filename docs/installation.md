# Installation Guide

## Prerequisites

Before installing the Llama-Swap SwiftBar Plugin, ensure you have:

1. **macOS 12.0 or later**
2. **SwiftBar** installed ([Download from swiftbar.app](https://swiftbar.app))
3. **Llama-Swap service** installed and configured
4. **Rust toolchain** (only for building from source)

## Quick Install (Pre-built Binary)

### 1. Download the Latest Release

Download the latest `llama-swap-swiftbar` binary from the [releases page](https://github.com/your-org/llama-swap-swiftbar/releases).

### 2. Install the Plugin

```bash
# Create SwiftBar plugins directory if it doesn't exist
mkdir -p "$HOME/Library/Application Support/SwiftBar/Plugins"

# Copy the binary with correct naming
cp llama-swap-swiftbar "$HOME/Library/Application Support/SwiftBar/Plugins/llama-swap.5s.o"

# Make it executable
chmod +x "$HOME/Library/Application Support/SwiftBar/Plugins/llama-swap.5s.o"

# Set streaming mode
xattr -w com.ameba.SwiftBar.type streamable "$HOME/Library/Application Support/SwiftBar/Plugins/llama-swap.5s.o"
```

### 3. Refresh SwiftBar

Either:
- Click the SwiftBar icon in the menu bar and select "Refresh All"
- Or run: `osascript -e 'tell application "SwiftBar" to refresh'`

## Building from Source

### 1. Clone the Repository

```bash
git clone https://github.com/your-org/llama-swap-swiftbar.git
cd llama-swap-swiftbar
```

### 2. Build the Plugin

```bash
# Install Rust if not already installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build in release mode
cargo build --release
```

### 3. Install Using the Script

```bash
./install.sh
```

Or manually:

```bash
cp target/release/llama-swap-swiftbar "$HOME/Library/Application Support/SwiftBar/Plugins/llama-swap.5s.o"
chmod +x "$HOME/Library/Application Support/SwiftBar/Plugins/llama-swap.5s.o"
xattr -w com.ameba.SwiftBar.type streamable "$HOME/Library/Application Support/SwiftBar/Plugins/llama-swap.5s.o"
```

## Configuration

### Environment Variables

The plugin can be configured via environment variables set in SwiftBar:

1. Open SwiftBar preferences
2. Select the Llama-Swap plugin
3. Add environment variables:

- `LLAMA_SWAP_PORT`: API port (default: 8080)
- `LLAMA_SWAP_LABEL`: LaunchAgent label (default: com.llamaswap.service)
- `LLAMA_SWAP_LOG`: Log file path (default: ~/Library/Logs/LlamaSwap.log)
- `LLAMA_SWAP_DEBUG`: Enable debug logging (set to any value)

### File Locations

The plugin expects:
- **Log file**: `~/Library/Logs/LlamaSwap.log`
- **Config file**: `~/.llamaswap/config.yaml`
- **Metrics cache**: `~/Library/Application Support/SwiftBar/PluginData/llama-swap-metrics.json`

## Verification

After installation, verify the plugin is working:

1. **Check Menu Bar**: You should see the Llama-Swap icon
2. **Click the Icon**: The dropdown menu should appear
3. **Check Status**: Icon should show 🟢 (green) if service is running, 🔴 (red) if stopped

## Troubleshooting

### Plugin Not Appearing

1. Check SwiftBar is running
2. Verify plugin is in correct directory:
   ```bash
   ls -la "$HOME/Library/Application Support/SwiftBar/Plugins/llama-swap.5s.o"
   ```
3. Check file permissions and attributes:
   ```bash
   # Should be executable
   ls -l "$HOME/Library/Application Support/SwiftBar/Plugins/llama-swap.5s.o"
   
   # Should show streamable type
   xattr -l "$HOME/Library/Application Support/SwiftBar/Plugins/llama-swap.5s.o"
   ```

### Service Control Not Working

1. Verify Llama-Swap LaunchAgent is installed:
   ```bash
   launchctl list | grep llamaswap
   ```
2. Check plugin can access launchctl:
   ```bash
   # Test command manually
   launchctl start com.llamaswap.service
   ```

### Metrics Not Showing

1. Verify Llama-Swap API is accessible:
   ```bash
   curl http://127.0.0.1:8080/metrics
   ```
2. Check for network/firewall issues
3. Verify correct port in configuration

### Debug Mode

Enable debug logging:

1. Set `LLAMA_SWAP_DEBUG=1` in SwiftBar environment variables
2. Check debug log:
   ```bash
   tail -f /tmp/llama-swap-plugin.log
   ```

## Uninstallation

To remove the plugin:

```bash
# Remove plugin file
rm "$HOME/Library/Application Support/SwiftBar/Plugins/llama-swap.5s.o"

# Remove cached data (optional)
rm "$HOME/Library/Application Support/SwiftBar/PluginData/llama-swap-metrics.json"

# Refresh SwiftBar
osascript -e 'tell application "SwiftBar" to refresh'
```

## Support

For issues or questions:
- Check the [FAQ](https://github.com/your-org/llama-swap-swiftbar/wiki/FAQ)
- Open an [issue](https://github.com/your-org/llama-swap-swiftbar/issues)
- Join our [Discord](https://discord.gg/llamaswap)