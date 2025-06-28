# Llama-Swap SwiftBar Plugin

## Project Purpose
This is a macOS menu bar plugin for SwiftBar that monitors and controls the Llama-Swap service. It provides real-time metrics, status monitoring, and control capabilities for a local AI model service.

## Tech Stack
- **Language**: Rust (edition 2021)
- **Menu Bar Integration**: bitbar crate (v0.7)
- **HTTP Client**: reqwest (blocking mode)
- **JSON**: serde/serde_json
- **Image Processing**: image crate for generating sparkline charts
- **System Info**: sysinfo crate (Apple-specific features)

## Key Features
- Real-time monitoring of AI model metrics (tokens per second, memory usage)
- Visual sparkline charts in the menu bar
- Service control (start/stop)
- Adaptive polling based on service state
- Graceful error handling and offline states

## Architecture
The plugin operates as a streaming application that continuously polls the Llama-Swap API and updates the SwiftBar menu. It maintains a history of metrics for charting and adapts its polling frequency based on the service state.