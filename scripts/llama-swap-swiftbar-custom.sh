#!/bin/zsh
#<swiftbar.type>streamable</swiftbar.type>
#<swiftbar.hideAbout>true</swiftbar.hideAbout>
#<swiftbar.hideRunInTerminal>true</swiftbar.hideRunInTerminal>
#<swiftbar.hideLastUpdated>true</swiftbar.hideLastUpdated>
#<swiftbar.hideDisablePlugin>true</swiftbar.hideDisablePlugin>
#<swiftbar.hideSwiftBar>true</swiftbar.hideSwiftBar>

# Llama-Swap SwiftBar Plugin Custom Configuration
# 
# This script demonstrates advanced configuration options and custom
# environment variables for specific use cases.
#
# Installation:
# 1. Download the appropriate binary for your architecture
# 2. Place it in the path specified by BINARY_PATH below
# 3. Customize the environment variables for your setup
# 4. Copy this script to your SwiftBar plugins directory
# 5. Make it executable: chmod +x llama-swap-swiftbar-custom.sh

# Configuration
BINARY_PATH="$HOME/.local/bin/llama-swap-swiftbar"

# Custom API Configuration
export LLAMA_SWAP_API_BASE_URL="http://192.168.1.100"  # Remote Llama-Swap instance
export LLAMA_SWAP_API_PORT="8080"                      # Custom port
export LLAMA_SWAP_API_TIMEOUT_SECS="3"                 # Longer timeout for remote

# Custom UI Configuration  
export LLAMA_SWAP_CHART_WIDTH="80"                     # Wider charts
export LLAMA_SWAP_CHART_HEIGHT="30"                    # Taller charts
export LLAMA_SWAP_HISTORY_SIZE="600"                   # 10 minutes of history

# Debug Configuration
export LLAMA_SWAP_DEBUG="false"                        # Disable debug for production
export LLAMA_SWAP_STREAMING_MODE="true"                # Keep streaming enabled

# Advanced: Custom file paths
# export LLAMA_SWAP_LOG_FILE_PATH="~/Documents/llama-swap.log"
# export LLAMA_SWAP_CONFIG_FILE_PATH="~/Documents/llama-swap-config.yaml"

# Check if binary exists
if [[ ! -x "$BINARY_PATH" ]]; then
    echo "⚠️ Custom Config"
    echo "---"
    echo "Binary not found: $BINARY_PATH"
    echo "Download from releases and update BINARY_PATH"
    echo "Current config:"
    echo "  API: $LLAMA_SWAP_API_BASE_URL:$LLAMA_SWAP_API_PORT"
    echo "  Charts: ${LLAMA_SWAP_CHART_WIDTH}x${LLAMA_SWAP_CHART_HEIGHT}"
    echo "  History: $LLAMA_SWAP_HISTORY_SIZE samples"
    exit 1
fi

# Execute the binary with all arguments
exec "$BINARY_PATH" "$@"