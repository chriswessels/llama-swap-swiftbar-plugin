#!/bin/zsh
#<swiftbar.type>streamable</swiftbar.type>
#<swiftbar.hideAbout>true</swiftbar.hideAbout>
#<swiftbar.hideRunInTerminal>true</swiftbar.hideRunInTerminal>
#<swiftbar.hideLastUpdated>true</swiftbar.hideLastUpdated>
#<swiftbar.hideDisablePlugin>true</swiftbar.hideDisablePlugin>
#<swiftbar.hideSwiftBar>true</swiftbar.hideSwiftBar>

# Llama-Swap SwiftBar Plugin Wrapper
# 
# This script wraps the llama-swap-swiftbar binary and allows for easy
# configuration via environment variables and SwiftBar annotations.
#
# Installation:
# 1. Download the appropriate binary for your architecture
# 2. Place it in the same directory as this script or in your PATH
# 3. Update BINARY_PATH below to point to your binary
# 4. Copy this script to your SwiftBar plugins directory
# 5. Make it executable: chmod +x llama-swap-swiftbar.sh

# Configuration
BINARY_PATH="$HOME/.local/bin/llama-swap-swiftbar"

# Environment variable overrides (uncomment and modify as needed)
# export LLAMA_SWAP_API_BASE_URL="http://127.0.0.1"
# export LLAMA_SWAP_API_PORT="45786"
# export LLAMA_SWAP_API_TIMEOUT_SECS="1"
# export LLAMA_SWAP_STREAMING_MODE="true"
# export LLAMA_SWAP_CHART_WIDTH="60"
# export LLAMA_SWAP_CHART_HEIGHT="20"
# export LLAMA_SWAP_HISTORY_SIZE="300"
# export LLAMA_SWAP_DEBUG="false"

# Check if binary exists
if [[ ! -x "$BINARY_PATH" ]]; then
    echo "❌ Binary not found"
    echo "---"
    echo "Expected binary at: $BINARY_PATH"
    echo "Please download from: https://github.com/your-org/llama-swap-swiftbar-plugin/releases"
    echo "Or update BINARY_PATH in this script"
    exit 1
fi

# Execute the binary with all arguments
exec "$BINARY_PATH" "$@"