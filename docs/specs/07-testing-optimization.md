# Phase 7: Testing & Optimization Specification

## Overview

This final phase focuses on comprehensive testing, performance optimization, and deployment preparation to ensure the plugin is production-ready.

## Goals

- Implement comprehensive test suite
- Optimize performance and resource usage
- Create deployment pipeline
- Document operational procedures

## Testing Strategy

### 7.1 Unit Testing Framework

Create a comprehensive test structure:

```rust
// In src/lib.rs (new file to expose modules for testing):

pub mod constants;
pub mod models;
pub mod menu;
pub mod metrics;
pub mod charts;
pub mod icons;
pub mod commands;
pub mod persistence;

#[cfg(test)]
mod tests {
    use super::*;
    
    /// Test fixture for creating consistent test states
    pub struct TestFixture {
        pub state: crate::PluginState,
        pub mock_metrics: Vec<models::Metrics>,
    }
    
    impl TestFixture {
        pub fn new() -> Self {
            Self {
                state: create_test_state(),
                mock_metrics: create_mock_metrics_sequence(),
            }
        }
        
        pub fn with_running_service(mut self) -> Self {
            self.state.current_status = models::ServiceStatus::Running;
            self
        }
        
        pub fn with_metrics_history(mut self) -> Self {
            for metrics in &self.mock_metrics {
                self.state.metrics_history.push(metrics);
            }
            self
        }
    }
}
```

### 7.2 Integration Tests

Create tests/integration_test.rs:

```rust
use llama_swap_swiftbar::*;
use std::process::Command;
use std::time::Duration;

#[test]
fn test_full_menu_generation() {
    let mut state = create_running_state_with_data();
    
    // Generate menu multiple times to test stability
    for _ in 0..10 {
        let menu = menu::build_menu(&state);
        let output = format!("{}", menu);
        
        // Verify essential elements
        assert!(output.contains("TPS"));
        assert!(output.contains("Memory"));
        assert!(output.contains("Stop Service"));
        
        // Verify no malformed output
        assert!(!output.contains("Error"));
        assert!(!output.contains("panic"));
    }
}

#[test]
fn test_streaming_output_format() {
    // Test that streaming output is properly formatted
    let output1 = "Menu 1";
    let output2 = "Menu 2";
    
    let streaming = format!("{}\n~~~\n{}", output1, output2);
    let parts: Vec<&str> = streaming.split("~~~").collect();
    
    assert_eq!(parts.len(), 2);
    assert!(parts[0].trim() == output1);
    assert!(parts[1].trim() == output2);
}

#[test]
fn test_command_execution() {
    // Test that commands can be executed without panic
    let commands = vec!["do_start", "do_stop", "view_logs"];
    
    for cmd in commands {
        // Should not panic even if service isn't installed
        let _ = commands::handle_command(cmd);
    }
}
```

### 7.3 Performance Testing

Create benchmarks/performance.rs:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use llama_swap_swiftbar::*;

fn benchmark_menu_generation(c: &mut Criterion) {
    let state = create_state_with_full_history();
    
    c.bench_function("menu_generation", |b| {
        b.iter(|| {
            let menu = menu::build_menu(black_box(&state));
            black_box(menu);
        });
    });
}

fn benchmark_chart_generation(c: &mut Criterion) {
    let data = create_test_data(60);
    
    c.bench_function("sparkline_generation", |b| {
        b.iter(|| {
            let chart = charts::generate_sparkline(
                black_box(&data),
                (255, 0, 0),
                60,
                15
            );
            black_box(chart);
        });
    });
}

fn benchmark_icon_generation(c: &mut Criterion) {
    c.bench_function("status_icon", |b| {
        b.iter(|| {
            let icon = icons::generate_status_icon(
                black_box(ServiceStatus::Running)
            );
            black_box(icon);
        });
    });
}

criterion_group!(benches, 
    benchmark_menu_generation,
    benchmark_chart_generation,
    benchmark_icon_generation
);
criterion_main!(benches);
```

Add to Cargo.toml:
```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "performance"
harness = false
```

### 7.4 Mock Service for Testing

Create a mock Llama-Swap service for testing:

```python
#!/usr/bin/env python3
# tools/mock_service.py

from http.server import HTTPServer, BaseHTTPRequestHandler
import json
import random
import time

class MockLlamaSwapHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == '/metrics':
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.end_headers()
            
            # Generate realistic-looking metrics
            metrics = {
                'tps': random.uniform(30, 60) + random.gauss(0, 5),
                'memory_bytes': int(1.5e9 + random.gauss(0, 1e8)),
                'cache_hits': random.randint(900, 1100),
                'cache_misses': random.randint(40, 60),
            }
            
            self.wfile.write(json.dumps(metrics).encode())
        
        elif self.path == '/health':
            self.send_response(200)
            self.end_headers()
            self.wfile.write(b'OK')
        
        else:
            self.send_response(404)
            self.end_headers()

if __name__ == '__main__':
    server = HTTPServer(('127.0.0.1', 8080), MockLlamaSwapHandler)
    print("Mock Llama-Swap service running on http://127.0.0.1:8080")
    server.serve_forever()
```

## Performance Optimization

### 7.5 Memory Optimization

Implement zero-copy where possible:

```rust
// In src/optimization.rs:

use std::borrow::Cow;

/// Optimize string allocations in menu generation
pub trait OptimizedMenuItem {
    fn new_static(text: &'static str) -> MenuItem {
        MenuItem::new(text)
    }
    
    fn new_cow<'a>(text: impl Into<Cow<'a, str>>) -> MenuItem {
        MenuItem::new(text.into())
    }
}

/// Reuse allocations for chart generation
pub struct ChartBuffer {
    image_buffer: Vec<u8>,
    work_buffer: Vec<f64>,
}

impl ChartBuffer {
    pub fn new() -> Self {
        Self {
            image_buffer: Vec::with_capacity(2000), // Typical chart size
            work_buffer: Vec::with_capacity(60),
        }
    }
    
    pub fn generate_chart(&mut self, data: &[f64]) -> Result<&[u8]> {
        self.work_buffer.clear();
        self.work_buffer.extend_from_slice(data);
        
        // Generate chart using buffers
        // ...
        
        Ok(&self.image_buffer)
    }
}
```

### 7.6 CPU Optimization

Profile and optimize hot paths:

```rust
// In src/charts.rs, optimize line drawing:

/// Optimized line drawing using integer arithmetic
fn draw_line_optimized(
    img: &mut RgbaImage,
    (x0, y0): (u32, u32),
    (x1, y1): (u32, u32),
    color: (u8, u8, u8),
) {
    // Pre-calculate pixel color
    let pixel = Rgba([color.0, color.1, color.2, 255]);
    
    // Use integer-only Bresenham's algorithm
    let dx = (x1 as i32 - x0 as i32).abs();
    let dy = (y1 as i32 - y0 as i32).abs();
    
    // Early exit for single pixel
    if dx == 0 && dy == 0 {
        img.put_pixel(x0, y0, pixel);
        return;
    }
    
    // Optimize vertical/horizontal lines
    if dx == 0 {
        let (y_start, y_end) = if y0 < y1 { (y0, y1) } else { (y1, y0) };
        for y in y_start..=y_end {
            img.put_pixel(x0, y, pixel);
        }
        return;
    }
    
    if dy == 0 {
        let (x_start, x_end) = if x0 < x1 { (x0, x1) } else { (x1, x0) };
        for x in x_start..=x_end {
            img.put_pixel(x, y0, pixel);
        }
        return;
    }
    
    // General case - optimized Bresenham
    // ... existing implementation
}
```

### 7.7 Binary Size Optimization

Reduce binary size:

```toml
# In Cargo.toml:

[profile.release]
opt-level = "z"          # Optimize for size
lto = true               # Link-time optimization
codegen-units = 1        # Better optimization
strip = true             # Strip symbols
panic = "abort"          # No unwinding

[dependencies]
# Use minimal features
reqwest = { version = "0.11", default-features = false, features = ["blocking", "json", "rustls-tls-native-roots"] }

# Consider alternatives for smaller size
# ureq = "2.0"  # Smaller HTTP client
```

Build script for size optimization:

```bash
#!/bin/bash
# build_release.sh

# Build with maximum optimization
cargo build --release

# Additional stripping (macOS)
strip target/release/llama-swap-swiftbar

# Check size
ls -lh target/release/llama-swap-swiftbar

# Optional: Use upx for compression
# upx --best target/release/llama-swap-swiftbar
```

## Deployment

### 7.8 Installation Script

Create install.sh:

```bash
#!/bin/bash
# Llama-Swap SwiftBar Plugin Installer

set -e

PLUGIN_NAME="llama-swap.5s.o"
SWIFTBAR_DIR="$HOME/Library/Application Support/SwiftBar/Plugins"
BINARY_PATH="target/release/llama-swap-swiftbar"

echo "🦙 Llama-Swap SwiftBar Plugin Installer"
echo "======================================"

# Check if SwiftBar is installed
if ! [ -d "$SWIFTBAR_DIR" ]; then
    echo "❌ SwiftBar plugins directory not found."
    echo "Please install SwiftBar first: https://swiftbar.app"
    exit 1
fi

# Build the plugin
echo "📦 Building plugin..."
cargo build --release

# Check build succeeded
if ! [ -f "$BINARY_PATH" ]; then
    echo "❌ Build failed. Please check error messages above."
    exit 1
fi

# Copy to SwiftBar plugins directory
echo "📋 Installing plugin..."
cp "$BINARY_PATH" "$SWIFTBAR_DIR/$PLUGIN_NAME"
chmod +x "$SWIFTBAR_DIR/$PLUGIN_NAME"

# Set streaming attribute
echo "⚙️ Configuring streaming mode..."
xattr -w com.ameba.SwiftBar.type streamable "$SWIFTBAR_DIR/$PLUGIN_NAME"

# Refresh SwiftBar
echo "🔄 Refreshing SwiftBar..."
osascript -e 'tell application "SwiftBar" to refresh'

echo "✅ Installation complete!"
echo ""
echo "The plugin should now appear in your menu bar."
echo "If not, try manually refreshing SwiftBar."
```

### 7.9 Release Checklist

Create RELEASE.md:

```markdown
# Release Checklist

## Pre-release
- [ ] Update version in Cargo.toml
- [ ] Update CHANGELOG.md
- [ ] Run full test suite: `cargo test`
- [ ] Run benchmarks: `cargo bench`
- [ ] Check for warnings: `cargo clippy`
- [ ] Format code: `cargo fmt`

## Build
- [ ] Clean build: `cargo clean && cargo build --release`
- [ ] Test binary size < 5MB
- [ ] Test on macOS versions: 12, 13, 14
- [ ] Test with SwiftBar 1.4+

## Documentation
- [ ] Update README.md
- [ ] Update installation instructions
- [ ] Generate API docs: `cargo doc`
- [ ] Update screenshots

## Release
- [ ] Tag version: `git tag v0.1.0`
- [ ] Create GitHub release
- [ ] Upload binary
- [ ] Update Homebrew formula (if applicable)

## Post-release
- [ ] Announce on forums/social media
- [ ] Monitor for issues
- [ ] Update project board
```

## Monitoring and Debugging

### 7.10 Debug Mode

Add debug features:

```rust
// In src/debug.rs:

use std::fs::OpenOptions;
use std::io::Write;

pub struct DebugLogger {
    enabled: bool,
    file: Option<std::fs::File>,
}

impl DebugLogger {
    pub fn new() -> Self {
        let enabled = std::env::var("LLAMA_SWAP_DEBUG").is_ok();
        
        let file = if enabled {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/llama-swap-plugin.log")
                .ok()
        } else {
            None
        };
        
        Self { enabled, file }
    }
    
    pub fn log(&mut self, message: &str) {
        if let Some(file) = &mut self.file {
            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
            writeln!(file, "[{}] {}", timestamp, message).ok();
        }
    }
}

// Usage:
static mut DEBUG_LOGGER: Option<DebugLogger> = None;

pub fn debug_log(message: &str) {
    unsafe {
        if let Some(logger) = &mut DEBUG_LOGGER {
            logger.log(message);
        }
    }
}
```

### 7.11 Error Reporting

Implement telemetry for error tracking:

```rust
// In src/telemetry.rs:

pub fn report_error(error: &dyn std::error::Error) {
    // In production, could send to error tracking service
    eprintln!("Error: {}", error);
    
    // Log to file for debugging
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/llama-swap-errors.log")
    {
        writeln!(file, "{}: {}", chrono::Local::now(), error).ok();
    }
}
```

## Final Testing Protocol

### Manual Testing Checklist

1. **Installation**
   - [ ] Fresh install works
   - [ ] Update from previous version works
   - [ ] Uninstall removes all files

2. **Functionality**
   - [ ] Service starts/stops correctly
   - [ ] Metrics display accurately
   - [ ] Charts render properly
   - [ ] All menu items clickable

3. **Edge Cases**
   - [ ] Service not installed
   - [ ] API not responding
   - [ ] Network issues
   - [ ] High memory usage
   - [ ] Long running time (24+ hours)

4. **Performance**
   - [ ] CPU usage < 1% average
   - [ ] Memory usage < 50MB
   - [ ] Menu updates within 100ms
   - [ ] No memory leaks over time

## Conclusion

With testing and optimization complete, the Llama-Swap SwiftBar plugin is ready for deployment. The comprehensive test suite ensures reliability, while performance optimizations keep resource usage minimal. The deployment scripts and documentation make installation straightforward for end users.