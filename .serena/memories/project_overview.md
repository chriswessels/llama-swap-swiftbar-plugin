# Llama-Swap SwiftBar Plugin

## Project Purpose
This is a macOS menu bar plugin for SwiftBar that monitors and controls the Llama-Swap service. It provides real-time metrics, status monitoring, and control capabilities for a local AI model service with sophisticated state management.

## Tech Stack
- **Language**: Rust (edition 2021)
- **Menu Bar Integration**: bitbar crate (v0.7) with base64 and image features
- **HTTP Client**: reqwest (blocking mode) with JSON and rustls-tls
- **JSON**: serde/serde_json with derive features
- **Image Processing**: image crate + png crate for sparkline charts and icons
- **Signal Handling**: ctrlc for graceful shutdown
- **System Info**: sysinfo crate for memory and CPU monitoring
- **Data Structures**: circular-queue with serde support for metrics history
- **Base64**: base64 crate for image encoding

## Key Features
- **Real-time Monitoring**: AI model metrics (tokens per second, memory usage, queue processing)
- **Visual Charts**: Sparkline charts in menu bar showing historical data
- **Service Control**: Start/stop/restart Llama-Swap service via LaunchAgent
- **Adaptive Polling**: Intelligent polling frequency based on activity and state changes
- **State Management**: Sophisticated state machine tracking agent, service, and model states
- **Service Health**: Multi-layer service status tracking (plist, launchctl, process, API)
- **Error Handling**: Graceful degradation for offline/error states
- **Installation UX**: Automatic detection and installation of service components

## Architecture
The plugin operates as a streaming application with:
- **State Model**: Clear separation between agent state, display state, and polling modes
- **Metrics History**: Time-series data storage with 5-minute retention
- **Service Management**: Full LaunchAgent lifecycle management
- **Error Recovery**: Preserves historical metrics across service failures
- **Testing**: Comprehensive test suite for critical functionality