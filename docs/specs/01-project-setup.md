# Phase 1: Project Setup Specification

## Overview

This phase establishes the foundation for the Llama-Swap SwiftBar plugin, including project structure, dependencies, and core constants.

## Goals

- Initialize a Rust project with optimal configuration
- Set up all required dependencies with correct features
- Define project constants and configuration
- Prepare embedded assets

## Implementation Steps

### 1.1 Create Cargo Project

```bash
cargo new llama-swap-swiftbar --bin
cd llama-swap-swiftbar
```

### 1.2 Configure Cargo.toml

```toml
[package]
name = "llama-swap-swiftbar"
version = "0.1.0"
edition = "2021"
authors = ["Your Name <email@example.com>"]
description = "SwiftBar plugin for Llama-Swap service monitoring"

[dependencies]
# Menu bar integration - disable default tokio feature
bitbar = { version = "0.7", default-features = false, features = ["base64", "image"] }

# HTTP client for API calls - blocking mode only
reqwest = { version = "0.11", default-features = false, features = ["blocking", "json", "rustls-tls"] }

# JSON parsing
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Image processing for charts and icons
image = "0.24"

# Optional: for colored terminal output during debugging
# env_logger = "0.10"

[profile.release]
opt-level = "z"     # Optimize for size
lto = true          # Enable Link Time Optimization
codegen-units = 1   # Single codegen unit for better optimization
strip = true        # Strip symbols for smaller binary
panic = "abort"     # Smaller binary, no unwinding

# Development profile for faster compilation
[profile.dev]
opt-level = 0
debug = true
```

### 1.3 Project Structure

```
llama-swap-swiftbar/
├── Cargo.toml
├── src/
│   ├── main.rs           # Entry point and main loop
│   ├── constants.rs      # Configuration constants
│   ├── models.rs         # Data structures
│   ├── menu.rs          # Menu construction
│   ├── metrics.rs       # Metrics fetching and storage
│   ├── charts.rs        # Sparkline rendering
│   ├── icons.rs         # Icon generation
│   └── commands.rs      # Command handlers
├── assets/
│   └── llama-icon.png   # Base icon (16x16 or 20x20)
├── docs/
└── README.md
```

### 1.4 Core Constants (src/constants.rs)

```rust
// Service configuration
pub const LAUNCH_AGENT_LABEL: &str = "com.llamaswap.service";
pub const SERVICE_NAME: &str = "Llama-Swap";

// API configuration
pub const API_BASE_URL: &str = "http://127.0.0.1";
pub const API_PORT: u16 = 8080;
pub const API_TIMEOUT_SECS: u64 = 1;

// Update timing
pub const UPDATE_INTERVAL_SECS: u64 = 5;
pub const STREAMING_MODE: bool = true;

// Chart configuration
pub const CHART_WIDTH: u32 = 60;
pub const CHART_HEIGHT: u32 = 15;
pub const HISTORY_SIZE: usize = 60; // 5 minutes at 5-second intervals

// File paths (using home directory expansion)
pub const LOG_FILE_PATH: &str = "~/Library/Logs/LlamaSwap.log";
pub const CONFIG_FILE_PATH: &str = "~/.llamaswap/config.yaml";

// Colors (RGB)
pub const COLOR_RUNNING: (u8, u8, u8) = (0, 200, 83);    // Green
pub const COLOR_STOPPED: (u8, u8, u8) = (213, 0, 0);     // Red
pub const COLOR_TPS_LINE: (u8, u8, u8) = (0, 255, 127);  // Spring green
pub const COLOR_MEM_LINE: (u8, u8, u8) = (0, 191, 255);  // Deep sky blue
pub const COLOR_CACHE_LINE: (u8, u8, u8) = (255, 165, 0); // Orange

// Icon configuration
pub const ICON_SIZE: u32 = 20;
pub const STATUS_DOT_SIZE: u32 = 6;
pub const STATUS_DOT_OFFSET: u32 = 2; // From bottom-right corner
```

### 1.5 Base Models (src/models.rs)

```rust
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ServiceStatus {
    Running,
    Stopped,
    Unknown,
}

#[derive(Debug, Deserialize)]
pub struct MetricsResponse {
    pub tps: f64,
    #[serde(rename = "memory_bytes")]
    pub memory_bytes: u64,
    #[serde(rename = "cache_hits")]
    pub cache_hits: u64,
    #[serde(rename = "cache_misses")]
    pub cache_misses: u64,
    // Additional fields can be added as needed
}

#[derive(Debug)]
pub struct Metrics {
    pub tps: f64,
    pub memory_mb: f64,
    pub cache_hit_rate: f64,
}

impl From<MetricsResponse> for Metrics {
    fn from(resp: MetricsResponse) -> Self {
        let total_cache = resp.cache_hits + resp.cache_misses;
        let cache_hit_rate = if total_cache > 0 {
            (resp.cache_hits as f64 / total_cache as f64) * 100.0
        } else {
            0.0
        };

        Self {
            tps: resp.tps,
            memory_mb: resp.memory_bytes as f64 / 1_048_576.0, // Convert to MB
            cache_hit_rate,
        }
    }
}

#[derive(Debug, Default)]
pub struct MetricsHistory {
    pub tps: VecDeque<f64>,
    pub memory_mb: VecDeque<f64>,
    pub cache_hit_rate: VecDeque<f64>,
}

impl MetricsHistory {
    pub fn new() -> Self {
        Self {
            tps: VecDeque::with_capacity(crate::constants::HISTORY_SIZE),
            memory_mb: VecDeque::with_capacity(crate::constants::HISTORY_SIZE),
            cache_hit_rate: VecDeque::with_capacity(crate::constants::HISTORY_SIZE),
        }
    }

    pub fn push(&mut self, metrics: &Metrics) {
        // Add new values
        self.tps.push_back(metrics.tps);
        self.memory_mb.push_back(metrics.memory_mb);
        self.cache_hit_rate.push_back(metrics.cache_hit_rate);

        // Remove old values if over capacity
        if self.tps.len() > crate::constants::HISTORY_SIZE {
            self.tps.pop_front();
        }
        if self.memory_mb.len() > crate::constants::HISTORY_SIZE {
            self.memory_mb.pop_front();
        }
        if self.cache_hit_rate.len() > crate::constants::HISTORY_SIZE {
            self.cache_hit_rate.pop_front();
        }
    }

    pub fn clear(&mut self) {
        self.tps.clear();
        self.memory_mb.clear();
        self.cache_hit_rate.clear();
    }
}
```

### 1.6 Asset Embedding

Create a build script (build.rs) to embed the icon:

```rust
// build.rs
fn main() {
    println!("cargo:rerun-if-changed=assets/llama-icon.png");
}
```

In src/icons.rs:

```rust
// Embed the base icon at compile time
pub const BASE_ICON_BYTES: &[u8] = include_bytes!("../assets/llama-icon.png");
```

### 1.7 Initial Main Structure (src/main.rs)

```rust
mod constants;
mod models;
mod menu;
mod metrics;
mod charts;
mod icons;
mod commands;

use std::error::Error;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn main() -> Result<()> {
    // Initialize any logging for debugging
    // env_logger::init();

    // Check if running as a command (for menu item clicks)
    if let Some(command) = std::env::args().nth(1) {
        return commands::handle_command(&command);
    }

    // Otherwise, run in streaming mode
    if constants::STREAMING_MODE {
        run_streaming_mode()
    } else {
        run_once()
    }
}

fn run_streaming_mode() -> Result<()> {
    // TODO: Implement streaming loop
    todo!("Implement streaming mode")
}

fn run_once() -> Result<()> {
    // TODO: Implement single execution
    todo!("Implement single run mode")
}
```

## Testing

After setup, verify:

1. Project compiles: `cargo build`
2. Dependencies resolve correctly
3. Icon file exists in assets/
4. Release build works: `cargo build --release`

## Next Steps

With the project foundation in place, proceed to [Phase 2: Streaming Infrastructure](02-streaming-infrastructure.md) to implement the core update loop.