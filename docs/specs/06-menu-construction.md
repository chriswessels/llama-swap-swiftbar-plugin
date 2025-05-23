# Phase 6: Menu Construction Specification

## Overview

This phase brings together all components to build the complete menu interface, implementing proper menu hierarchy, conditional display logic, and polished user experience elements.

## Goals

- Build complete menu with all sections
- Implement conditional display based on service state
- Add user experience enhancements
- Optimize menu generation performance

## Implementation

### 6.1 Complete Menu Builder

Update src/menu.rs with full implementation:

```rust
use bitbar::{Menu, MenuItem, Command, Params};
use crate::{PluginState, icons, charts, constants};
use crate::models::{ServiceStatus, MetricStats};

/// Build the complete menu based on current state
pub fn build_menu(state: &PluginState) -> Menu {
    let mut menu = MenuBuilder::new();
    
    // Add title with status icon
    menu.add_title(state.current_status);
    
    // Add sections based on state
    menu.add_separator();
    menu.add_control_section(state.current_status);
    
    menu.add_separator();
    menu.add_file_section();
    
    // Only show metrics if service is running
    if state.current_status == ServiceStatus::Running && !state.metrics_history.tps.is_empty() {
        menu.add_separator();
        menu.add_metrics_section(&state.metrics_history);
    }
    
    // Add footer section
    menu.add_separator();
    menu.add_footer_section();
    
    menu.build()
}

/// Menu builder for cleaner construction
struct MenuBuilder {
    items: Vec<MenuItem>,
}

impl MenuBuilder {
    fn new() -> Self {
        Self { items: Vec::new() }
    }
    
    fn add_title(&mut self, status: ServiceStatus) {
        match icons::generate_status_icon(status) {
            Ok(icon) => {
                match icons::icon_to_menu_image(icon) {
                    Ok(menu_image) => {
                        self.items.push(MenuItem::new("").image(menu_image));
                    }
                    Err(e) => {
                        eprintln!("Failed to convert icon: {}", e);
                        self.add_text_title(status);
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to generate icon: {}", e);
                self.add_text_title(status);
            }
        }
    }
    
    fn add_text_title(&mut self, status: ServiceStatus) {
        let title = match status {
            ServiceStatus::Running => "🟢 Llama-Swap",
            ServiceStatus::Stopped => "🔴 Llama-Swap",
            ServiceStatus::Unknown => "⚪ Llama-Swap",
        };
        self.items.push(MenuItem::new(title));
    }
    
    fn add_separator(&mut self) {
        self.items.push(MenuItem::Sep);
    }
    
    fn add_control_section(&mut self, status: ServiceStatus) {
        match status {
            ServiceStatus::Running => {
                self.items.push(
                    MenuItem::new("🔴 Stop Service")
                        .command(Command::exec("do_stop"))
                        .shortcut("cmd+s")
                );
            }
            ServiceStatus::Stopped | ServiceStatus::Unknown => {
                self.items.push(
                    MenuItem::new("🟢 Start Service")
                        .command(Command::exec("do_start"))
                        .shortcut("cmd+s")
                );
            }
        }
        
        self.items.push(
            MenuItem::new("⟲ Restart Service")
                .command(Command::exec("do_restart"))
                .shortcut("cmd+r")
        );
    }
    
    fn add_file_section(&mut self) {
        // Use direct shell commands for file operations
        let log_path = expand_tilde(constants::LOG_FILE_PATH)
            .unwrap_or_else(|_| constants::LOG_FILE_PATH.to_string());
        let config_path = expand_tilde(constants::CONFIG_FILE_PATH)
            .unwrap_or_else(|_| constants::CONFIG_FILE_PATH.to_string());
        
        self.items.push(
            MenuItem::new("📄 View Logs")
                .command(Command::bash("/usr/bin/open", vec!["-t", &log_path]))
                .alternate(
                    MenuItem::new("📁 Show in Finder")
                        .command(Command::bash("/usr/bin/open", vec!["-R", &log_path]))
                )
        );
        
        self.items.push(
            MenuItem::new("⚙️ Edit Configuration")
                .command(Command::bash("/usr/bin/open", vec!["-t", &config_path]))
                .alternate(
                    MenuItem::new("📁 Show in Finder")
                        .command(Command::bash("/usr/bin/open", vec!["-R", &config_path]))
                )
        );
    }
    
    fn add_metrics_section(&mut self, history: &crate::models::MetricsHistory) {
        // Section header
        self.items.push(
            MenuItem::new("Performance Metrics")
                .color("#666666")
        );
        
        // TPS with sparkline and stats
        if let Some(item) = self.create_metric_item(
            "TPS",
            &history.tps,
            charts::generate_tps_sparkline,
            |v| format!("{:.1}", v),
            true, // show stats
        ) {
            self.items.push(item);
        }
        
        // Memory with sparkline
        if let Some(item) = self.create_metric_item(
            "Memory",
            &history.memory_mb,
            charts::generate_memory_sparkline,
            |v| format_memory(v),
            false,
        ) {
            self.items.push(item);
        }
        
        // Cache hit rate with sparkline
        if let Some(item) = self.create_metric_item(
            "Cache Hit Rate",
            &history.cache_hit_rate,
            charts::generate_cache_sparkline,
            |v| format!("{:.1}%", v),
            false,
        ) {
            self.items.push(item);
        }
        
        // Add statistics submenu
        self.add_stats_submenu(history);
    }
    
    fn create_metric_item<F, G>(
        &self,
        name: &str,
        data: &std::collections::VecDeque<crate::models::TimestampedValue>,
        chart_fn: F,
        format_fn: G,
        show_inline_stats: bool,
    ) -> Option<MenuItem>
    where
        F: Fn(&std::collections::VecDeque<f64>) -> crate::Result<image::DynamicImage>,
        G: Fn(f64) -> String,
    {
        let values = data.iter().map(|tv| tv.value).collect();
        let latest = data.back()?.value;
        
        let mut label = format!("{}: {}", name, format_fn(latest));
        
        // Add inline stats if requested
        if show_inline_stats && data.len() > 1 {
            let stats = crate::models::MetricsHistory::calculate_stats_static(data);
            label.push_str(&format!(" (avg: {:.1})", stats.mean));
        }
        
        let mut item = MenuItem::new(label);
        
        // Add sparkline chart
        if let Ok(chart) = chart_fn(&values) {
            if let Ok(chart_image) = chart.try_into() {
                item = item.image(chart_image);
            }
        }
        
        Some(item)
    }
    
    fn add_stats_submenu(&mut self, history: &crate::models::MetricsHistory) {
        let mut stats_items = vec![];
        
        // TPS statistics
        if !history.tps.is_empty() {
            let stats = crate::models::MetricsHistory::calculate_stats_static(&history.tps);
            stats_items.push(MenuItem::new("TPS Statistics").color("#666666"));
            stats_items.push(MenuItem::new(format!("  Average: {:.1}", stats.mean)));
            stats_items.push(MenuItem::new(format!("  Min: {:.1}", stats.min)));
            stats_items.push(MenuItem::new(format!("  Max: {:.1}", stats.max)));
            stats_items.push(MenuItem::new(format!("  Std Dev: {:.1}", stats.std_dev)));
        }
        
        if !stats_items.is_empty() {
            self.items.push(
                MenuItem::new("📊 View Statistics...")
                    .sub(stats_items)
            );
        }
    }
    
    fn add_footer_section(&mut self) {
        // Version and about info
        let version = env!("CARGO_PKG_VERSION");
        
        self.items.push(
            MenuItem::new(format!("Llama-Swap Plugin v{}", version))
                .color("#666666")
                .href("https://github.com/your-org/llama-swap-swiftbar")
        );
        
        // Add refresh option for debugging
        if cfg!(debug_assertions) {
            self.items.push(
                MenuItem::new("🔄 Force Refresh")
                    .command(Command::refresh())
            );
        }
    }
    
    fn build(self) -> Menu {
        Menu(self.items)
    }
}

/// Helper to format memory values
fn format_memory(mb: f64) -> String {
    if mb < 1024.0 {
        format!("{:.1} MB", mb)
    } else {
        format!("{:.2} GB", mb / 1024.0)
    }
}

/// Expand tilde to home directory
fn expand_tilde(path: &str) -> crate::Result<String> {
    if path.starts_with("~/") {
        let home = std::env::var("HOME")
            .map_err(|_| "Failed to get HOME directory")?;
        Ok(path.replacen("~", &home, 1))
    } else {
        Ok(path.to_string())
    }
}

/// Build an error menu for display when things go wrong
pub fn build_error_menu(message: &str) -> Result<Menu, std::fmt::Error> {
    Ok(Menu(vec![
        MenuItem::new("⚠️ Plugin Error"),
        MenuItem::Sep,
        MenuItem::new(message)
            .color("#ff0000")
            .font("Menlo", 11),
        MenuItem::Sep,
        MenuItem::new("🔄 Retry")
            .command(Command::refresh()),
    ]))
}

/// Build a minimal menu for when service is not installed
pub fn build_not_installed_menu() -> Menu {
    Menu(vec![
        MenuItem::new("⚪ Llama-Swap"),
        MenuItem::Sep,
        MenuItem::new("Service not installed")
            .color("#666666"),
        MenuItem::new("Visit documentation...")
            .href("https://github.com/your-org/llama-swap"),
    ])
}
```

### 6.2 Command System Enhancement

Update command handling for better integration:

```rust
// In src/commands.rs, enhance with bitbar attributes:

use bitbar::attr::{Command as Cmd};

/// Create proper bitbar commands
impl CommandBuilder {
    pub fn restart(subcommand: &str) -> Cmd {
        // Get the plugin path
        let plugin_path = std::env::current_exe()
            .ok()
            .and_then(|p| p.to_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "llama-swap-swiftbar".to_string());
        
        Cmd::new(&plugin_path)
            .param(subcommand)
            .terminal(false)
    }
    
    pub fn bash(cmd: &str, args: Vec<&str>) -> Cmd {
        let mut command = Cmd::new(cmd);
        for arg in args {
            command = command.param(arg);
        }
        command.terminal(false)
    }
}
```

### 6.3 Dynamic Menu Features

Add context-aware menu items:

```rust
// In menu.rs, add dynamic features:

impl MenuBuilder {
    fn add_conditional_items(&mut self, state: &PluginState) {
        // Add alerts for anomalies
        if let Some(anomaly) = self.check_for_anomalies(&state.metrics_history) {
            self.items.push(
                MenuItem::new(format!("⚠️ {}", anomaly))
                    .color("#ff9900")
            );
            self.add_separator();
        }
        
        // Add quick actions based on metrics
        if let Some(&latest_mem) = state.metrics_history.memory_mb.back() {
            if latest_mem > 4096.0 { // Over 4GB
                self.items.push(
                    MenuItem::new("⚠️ High memory usage")
                        .color("#ff6600")
                        .sub(vec![
                            MenuItem::new("Restart service to free memory")
                                .command(Command::exec("do_restart")),
                            MenuItem::new("View memory details...")
                                .command(Command::exec("show_memory_details")),
                        ])
                );
            }
        }
    }
    
    fn check_for_anomalies(&self, history: &crate::models::MetricsHistory) -> Option<String> {
        // Simple anomaly detection
        if let Some(&latest_tps) = history.tps.back() {
            if latest_tps == 0.0 && history.tps.len() > 5 {
                // Check if TPS dropped to zero suddenly
                let previous_avg: f64 = history.tps.iter()
                    .rev()
                    .skip(1)
                    .take(5)
                    .map(|tv| tv.value)
                    .sum::<f64>() / 5.0;
                
                if previous_avg > 10.0 {
                    return Some("TPS dropped to zero".to_string());
                }
            }
        }
        
        None
    }
}
```

### 6.4 Keyboard Shortcuts

Add keyboard shortcut support:

```rust
// In menu items:

self.items.push(
    MenuItem::new("🔴 Stop Service")
        .command(Command::exec("do_stop"))
        .shortcut("cmd+s")  // SwiftBar supports keyboard shortcuts
);

self.items.push(
    MenuItem::new("📊 Toggle Statistics")
        .command(Command::exec("toggle_stats"))
        .shortcut("cmd+t")
);
```

### 6.5 Theme-Aware Colors

Add support for light/dark mode:

```rust
// In src/theme.rs (new file):

use bitbar::Theme;

pub struct Colors {
    pub text_secondary: &'static str,
    pub warning: &'static str,
    pub error: &'static str,
    pub success: &'static str,
}

impl Colors {
    pub fn for_theme(theme: Theme) -> Self {
        match theme {
            Theme::Light => Self {
                text_secondary: "#666666",
                warning: "#ff9900",
                error: "#cc0000",
                success: "#00aa00",
            },
            Theme::Dark => Self {
                text_secondary: "#999999",
                warning: "#ffaa00",
                error: "#ff3333",
                success: "#00cc00",
            },
        }
    }
}

// Use in menu:
let colors = Colors::for_theme(Theme::current());
self.items.push(
    MenuItem::new("Secondary text")
        .color(colors.text_secondary)
);
```

### 6.6 Menu Performance Optimization

Optimize menu generation for speed:

```rust
// In src/menu.rs:

use std::sync::OnceLock;

static MENU_CACHE: OnceLock<MenuCache> = OnceLock::new();

struct MenuCache {
    static_items: Vec<MenuItem>,
}

impl MenuCache {
    fn get() -> &'static Self {
        MENU_CACHE.get_or_init(|| Self {
            static_items: Self::build_static_items(),
        })
    }
    
    fn build_static_items() -> Vec<MenuItem> {
        vec![
            // Pre-build items that never change
            MenuItem::new("📄 View Logs")
                .command(Command::exec("view_logs")),
            MenuItem::new("⚙️ Edit Configuration")
                .command(Command::exec("view_config")),
        ]
    }
}

// Use cached items in menu builder:
fn add_file_section(&mut self) {
    let cache = MenuCache::get();
    self.items.extend(cache.static_items.iter().cloned());
}
```

## Testing Menu Construction

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_menu_with_running_service() {
        let state = create_test_state(ServiceStatus::Running);
        let menu = build_menu(&state);
        
        // Should contain stop option
        assert!(menu.0.iter().any(|item| 
            item.text.contains("Stop")
        ));
    }
    
    #[test]
    fn test_menu_with_stopped_service() {
        let state = create_test_state(ServiceStatus::Stopped);
        let menu = build_menu(&state);
        
        // Should contain start option
        assert!(menu.0.iter().any(|item| 
            item.text.contains("Start")
        ));
        
        // Should not show metrics
        assert!(!menu.0.iter().any(|item| 
            item.text.contains("TPS")
        ));
    }
    
    fn create_test_state(status: ServiceStatus) -> PluginState {
        PluginState {
            current_status: status,
            metrics_history: MetricsHistory::new(),
            // ... other fields
        }
    }
}
```

### Visual Testing

Create a test binary for visual inspection:

```rust
// In examples/menu_preview.rs:

fn main() {
    let state = create_mock_state();
    let menu = llama_swap_swiftbar::menu::build_menu(&state);
    
    // Print menu for manual inspection
    println!("{}", menu);
    
    // Also save to file for SwiftBar testing
    std::fs::write("test_menu.txt", format!("{}", menu)).unwrap();
}
```

## User Experience Enhancements

### Progressive Disclosure
- Basic info in main menu
- Detailed stats in submenus
- Advanced options hidden behind alt-click

### Visual Hierarchy
- Clear section separation
- Consistent icon usage
- Proper text formatting

### Responsive Feedback
- Immediate visual response to clicks
- Loading states for slow operations
- Error messages with recovery options

## Next Steps

With menu construction complete, proceed to [Phase 7: Testing & Optimization](07-testing-optimization.md) for final polish and deployment preparation.