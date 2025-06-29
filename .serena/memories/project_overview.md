# Llama-Swap SwiftBar Plugin

## Project Purpose
This is a macOS menu bar plugin for SwiftBar that monitors and controls the Llama-Swap service - a multi-model AI inference service based on llama.cpp. The plugin provides real-time metrics monitoring, service lifecycle management, and visual analytics for AI model performance with sophisticated state management.

## What is Llama-Swap?
Llama-Swap is a multi-model AI inference service that can run multiple LLM (Large Language Model) instances simultaneously. It appears to be built on llama.cpp infrastructure and provides:
- Concurrent AI model hosting and inference
- Prometheus metrics for monitoring (tokens per second, queue processing, memory usage)
- Web UI for model management at `/ui/models`
- REST API for service control (`/running`, `/metrics`, `/unload` endpoints)
- Support for model loading/unloading and request queuing

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
- **Real-time Monitoring**: AI inference metrics (tokens per second, memory usage, queue processing)
- **Multi-Model Support**: Tracks multiple AI models running concurrently
- **Visual Charts**: Sparkline charts in menu bar showing historical data
- **Service Control**: Start/stop/restart Llama-Swap service via LaunchAgent
- **Adaptive Polling**: Intelligent polling frequency based on activity and state changes
- **State Management**: Sophisticated state machine tracking agent, service, and model states
- **Service Health**: Multi-layer service status tracking (plist, launchctl, process, API)
- **Error Handling**: Graceful degradation for offline/error states
- **Installation UX**: Automatic detection and installation of service components
- **Web UI Integration**: Direct access to Llama-Swap web interface
- **System Monitoring**: CPU and memory usage tracking alongside AI metrics

## Architecture
The plugin operates as a streaming application with:
- **State Model**: Clear separation between agent state, display state, and polling modes
- **Metrics History**: Time-series data storage with 5-minute retention using CircularQueue
- **Service Management**: Full LaunchAgent lifecycle management for macOS integration
- **Error Recovery**: Preserves historical metrics across service failures
- **API Integration**: Monitors Llama-Swap via REST API and Prometheus metrics
- **Testing**: Comprehensive test suite for critical functionality