#!/bin/zsh
#<swiftbar.type>streamable</swiftbar.type>
#<swiftbar.hideAbout>true</swiftbar.hideAbout>
#<swiftbar.hideRunInTerminal>true</swiftbar.hideRunInTerminal>
#<swiftbar.hideLastUpdated>true</swiftbar.hideLastUpdated>
#<swiftbar.hideDisablePlugin>true</swiftbar.hideDisablePlugin>
#<swiftbar.hideSwiftBar>true</swiftbar.hideSwiftBar>

# Llama-Swap SwiftBar Plugin Development Wrapper
# 
# This script is for development purposes - it builds and runs the plugin
# from source using cargo. Useful for testing changes during development.
#
# Installation:
# 1. Clone the repository to a local directory
# 2. Update PROJECT_PATH below to point to your clone
# 3. Copy this script to your SwiftBar plugins directory
# 4. Make it executable: chmod +x llama-swap-swiftbar-dev.sh

# Configuration
PROJECT_PATH="$HOME/dev/llama-swap-swiftbar-plugin"

# Development environment variables
export LLAMA_SWAP_DEBUG="true"
export RUST_BACKTRACE="1"

# Custom configuration for development (uncomment as needed)
# export LLAMA_SWAP_API_PORT="8080"        # If using different port
# export LLAMA_SWAP_CHART_WIDTH="80"       # Larger charts for testing
# export LLAMA_SWAP_HISTORY_SIZE="600"     # More history for testing

# Check if project directory exists
if [[ ! -d "$PROJECT_PATH" ]]; then
    echo "❌ Project not found"
    echo "---"
    echo "Expected project at: $PROJECT_PATH"
    echo "Please clone from: https://github.com/your-org/llama-swap-swiftbar-plugin"
    echo "Or update PROJECT_PATH in this script"
    exit 1
fi

# Change to project directory and run with cargo
cd "$PROJECT_PATH" || exit 1

# Check if Cargo.toml exists
if [[ ! -f "Cargo.toml" ]]; then
    echo "❌ Not a Rust project"
    echo "---"
    echo "No Cargo.toml found in: $PROJECT_PATH"
    exit 1
fi

# Execute with cargo run for development
exec cargo run --profile release --bin llama-swap-swiftbar -- "$@"