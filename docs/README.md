# Llama-Swap SwiftBar Plugin Documentation

## Overview

The Llama-Swap SwiftBar Plugin is a native macOS menu bar application that provides real-time monitoring and control of the Llama-Swap background service. Built as a single, self-contained Rust binary, it integrates seamlessly with SwiftBar to deliver a responsive and informative user interface.

## Project Goals

1. **Real-time Monitoring**: Display live performance metrics with visual trends
2. **Service Control**: Start, stop, and restart the Llama-Swap service with one click
3. **Performance**: Minimal resource usage while providing continuous updates
4. **User Experience**: Clean, intuitive interface that follows macOS design patterns
5. **Reliability**: Robust error handling and graceful degradation

## Technology Stack

### Core Technologies
- **Rust**: Systems programming language for performance and safety
- **SwiftBar**: Menu bar app that displays script output
- **bitbar crate**: Rust library for BitBar/SwiftBar plugin development

### Key Dependencies
- **reqwest**: HTTP client for API communication (blocking mode)
- **serde/serde_json**: JSON parsing and serialization
- **image**: Image manipulation for chart generation
- **std::process**: System command execution

### Architecture Highlights
- **Streaming Mode**: Continuous process that pushes updates every 5 seconds
- **Single Binary**: No external dependencies or scripts required
- **Embedded Resources**: Icons and assets compiled into the binary
- **Command Pattern**: Click actions handled via bitbar command dispatch

## Features

### Menu Bar Display
- Two-icon status indicator (main icon + colored status dot)
- Live update without clicking
- Theme-aware rendering

### Dropdown Menu
- **Service Controls**: Start/Stop/Restart buttons
- **Quick Access**: View logs and configuration
- **Performance Metrics**: 
  - Transactions Per Second (TPS)
  - Memory Usage
  - Cache Hit Rate
  - Mini sparkline charts showing 5-minute trends

### Technical Features
- HTTP API integration for metrics
- LaunchAgent control via launchctl
- In-memory metrics history (60-sample circular buffer)
- Dynamic PNG chart generation
- Base64 image encoding for SwiftBar

## Documentation Structure

- **[Overview](README.md)**: This document
- **[Installation Guide](installation.md)**: Setup and configuration instructions
- **[Developer Guide](developer.md)**: Building and contributing
- **Specifications**:
  - [Phase 1: Project Setup](specs/01-project-setup.md)
  - [Phase 2: Streaming Infrastructure](specs/02-streaming-infrastructure.md)
  - [Phase 3: Visual Components](specs/03-visual-components.md)
  - [Phase 4: Service Integration](specs/04-service-integration.md)
  - [Phase 5: Data Management](specs/05-data-management.md)
  - [Phase 6: Menu Construction](specs/06-menu-construction.md)
  - [Phase 7: Testing & Optimization](specs/07-testing-optimization.md)

## Design Principles

1. **Simplicity**: Single binary, no configuration files required
2. **Performance**: Efficient resource usage, minimal CPU/memory footprint
3. **Reliability**: Graceful error handling, never crashes
4. **Maintainability**: Modular code structure, comprehensive documentation
5. **User-Centric**: Intuitive interface, responsive feedback

## Getting Started

For installation instructions, see the [Installation Guide](installation.md).

For development setup, see the [Developer Guide](developer.md).

For detailed implementation specifications, browse the [specs](specs/) directory.