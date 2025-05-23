use bitbar::{Menu, MenuItem, ContentItem, attr};
use crate::{icons, charts, constants};
use crate::models::{ServiceStatus, MetricsHistory};

/// Build the complete menu based on current state
pub fn build_menu(state: &crate::PluginState) -> crate::Result<String> {
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
    
    let built_menu = menu.build();
    Ok(built_menu.to_string())
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
                        let item = ContentItem::new("").image(menu_image).unwrap();
                        self.items.push(MenuItem::Content(item));
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
        self.items.push(MenuItem::Content(ContentItem::new(title)));
    }
    
    fn add_separator(&mut self) {
        self.items.push(MenuItem::Sep);
    }
    
    fn add_control_section(&mut self, status: ServiceStatus) {
        let exe = std::env::current_exe().unwrap();
        let exe_str = exe.to_str().unwrap();
        
        match status {
            ServiceStatus::Running => {
                let mut item = ContentItem::new("🔴 Stop Service");
                item = item.command(attr::Command::try_from((exe_str, "do_stop")).unwrap()).unwrap();
                // Note: SwiftBar doesn't support keyboard shortcuts in the same way as BitBar
                self.items.push(MenuItem::Content(item));
            }
            ServiceStatus::Stopped | ServiceStatus::Unknown => {
                let mut item = ContentItem::new("🟢 Start Service");
                item = item.command(attr::Command::try_from((exe_str, "do_start")).unwrap()).unwrap();
                self.items.push(MenuItem::Content(item));
            }
        }
        
        let mut restart = ContentItem::new("⟲ Restart Service");
        restart = restart.command(attr::Command::try_from((exe_str, "do_restart")).unwrap()).unwrap();
        self.items.push(MenuItem::Content(restart));
    }
    
    fn add_file_section(&mut self) {
        let exe = std::env::current_exe().unwrap();
        let exe_str = exe.to_str().unwrap();
        
        // Use direct shell commands for file operations
        let _log_path = expand_tilde(constants::LOG_FILE_PATH)
            .unwrap_or_else(|_| constants::LOG_FILE_PATH.to_string());
        let _config_path = expand_tilde(constants::CONFIG_FILE_PATH)
            .unwrap_or_else(|_| constants::CONFIG_FILE_PATH.to_string());
        
        // View logs command - use plugin executable with command
        let mut view_logs = ContentItem::new("📄 View Logs");
        view_logs = view_logs.command(attr::Command::try_from((exe_str, "view_logs")).unwrap()).unwrap();
        self.items.push(MenuItem::Content(view_logs));
        
        // Edit configuration command
        let mut edit_config = ContentItem::new("⚙️ Edit Configuration");
        edit_config = edit_config.command(attr::Command::try_from((exe_str, "view_config")).unwrap()).unwrap();
        self.items.push(MenuItem::Content(edit_config));
    }
    
    fn add_metrics_section(&mut self, history: &MetricsHistory) {
        // Check for anomalies first
        self.add_conditional_items(history);
        
        // Section header
        let mut header = ContentItem::new("Performance Metrics");
        header = header.color("#666666").unwrap();
        self.items.push(MenuItem::Content(header));
        
        // Generation tokens per second with sparkline
        if let Some(item) = self.create_metric_item(
            "Generation Speed",
            &history.tps,
            charts::generate_tps_sparkline,
            |v| format!("{:.1} tok/s", v),
            true, // show stats
        ) {
            self.items.push(item);
        }
        
        // Cache hit rate with sparkline
        if let Some(item) = self.create_metric_item(
            "Cache Hit Rate",
            &history.cache_hit_rate,
            charts::generate_cache_sparkline,
            |v| format!("{:.0}%", v),
            false,
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
            let stats = calculate_stats_for_data(data);
            label.push_str(&format!(" (avg: {:.1})", stats.mean));
        }
        
        let mut item = ContentItem::new(label);
        
        // Add sparkline chart
        if let Ok(chart) = chart_fn(&values) {
            if let Ok(chart_image) = icons::icon_to_menu_image(chart) {
                item = item.image(chart_image).unwrap();
            }
        }
        
        Some(MenuItem::Content(item))
    }
    
    fn add_stats_submenu(&mut self, history: &MetricsHistory) {
        let mut stats_items = vec![];
        
        // Token generation statistics
        if !history.tps.is_empty() {
            let stats = calculate_stats_for_data(&history.tps);
            let mut header = ContentItem::new("Generation Speed Statistics");
            header = header.color("#666666").unwrap();
            stats_items.push(MenuItem::Content(header));
            stats_items.push(MenuItem::Content(ContentItem::new(format!("  Avg Speed: {:.1} tok/s", stats.mean))));
            stats_items.push(MenuItem::Content(ContentItem::new(format!("  Min: {:.1} tok/s", stats.min))));
            stats_items.push(MenuItem::Content(ContentItem::new(format!("  Max: {:.1} tok/s", stats.max))));
            stats_items.push(MenuItem::Content(ContentItem::new(format!("  Std Dev: {:.1}", stats.std_dev))));
        }
        
        if !stats_items.is_empty() {
            let mut stats_item = ContentItem::new("📊 View Statistics...");
            stats_item = stats_item.sub(stats_items);
            self.items.push(MenuItem::Content(stats_item));
        }
    }
    
    fn add_footer_section(&mut self) {
        // Version and about info
        let version = env!("CARGO_PKG_VERSION");
        
        let mut version_item = ContentItem::new(format!("Llama-Swap Plugin v{}", version));
        version_item = version_item.color("#666666").unwrap();
        version_item = version_item.href("https://github.com/your-org/llama-swap-swiftbar").unwrap();
        self.items.push(MenuItem::Content(version_item));
        
        // Add refresh option for debugging
        if cfg!(debug_assertions) {
            let mut refresh_item = ContentItem::new("🔄 Force Refresh");
            refresh_item = refresh_item.refresh();
            self.items.push(MenuItem::Content(refresh_item));
        }
    }
    
    fn add_conditional_items(&mut self, history: &MetricsHistory) {
        let exe = std::env::current_exe().unwrap();
        let exe_str = exe.to_str().unwrap();
        
        // Add alerts for anomalies
        if let Some(anomaly) = self.check_for_anomalies(history) {
            let mut alert = ContentItem::new(format!("⚠️ {}", anomaly));
            alert = alert.color("#ff9900").unwrap();
            self.items.push(MenuItem::Content(alert));
            self.add_separator();
        }
        
        // Add quick actions based on metrics
        if let Some(latest_mem) = history.memory_mb.back() {
            if latest_mem.value > 4096.0 { // Over 4GB
                let mut high_mem_item = ContentItem::new("⚠️ High memory usage");
                high_mem_item = high_mem_item.color("#ff6600").unwrap();
                
                let mut restart_submenu = ContentItem::new("Restart service to free memory");
                restart_submenu = restart_submenu.command(attr::Command::try_from((exe_str, "do_restart")).unwrap()).unwrap();
                
                let mut mem_details = ContentItem::new("View memory details...");
                mem_details = mem_details.command(attr::Command::try_from((exe_str, "show_memory_details")).unwrap()).unwrap();
                
                high_mem_item = high_mem_item.sub(vec![
                    MenuItem::Content(restart_submenu),
                    MenuItem::Content(mem_details),
                ]);
                
                self.items.push(MenuItem::Content(high_mem_item));
                self.add_separator();
            }
        }
    }
    
    fn check_for_anomalies(&self, history: &MetricsHistory) -> Option<String> {
        // Check if token generation stopped suddenly
        if let Some(latest_tokens) = history.tps.back() {
            if latest_tokens.value == 0.0 && history.tps.len() > 5 {
                // Check if tokens were previously being generated
                let previous_avg: f64 = history.tps.iter()
                    .rev()
                    .skip(1)
                    .take(5)
                    .map(|tv| tv.value)
                    .sum::<f64>() / 5.0;
                
                if previous_avg > 10.0 {
                    return Some("Token generation stopped".to_string());
                }
            }
        }
        
        // Check for memory spike
        if history.memory_mb.len() > 10 {
            let recent_values: Vec<f64> = history.memory_mb.iter()
                .rev()
                .take(10)
                .map(|tv| tv.value)
                .collect();
            
            if let (Some(&latest), Some(&min)) = (recent_values.first(), recent_values.iter().min_by(|a, b| a.partial_cmp(b).unwrap())) {
                if latest > min * 2.0 && latest > 2048.0 {
                    return Some("Memory usage doubled".to_string());
                }
            }
        }
        
        // Check for low cache hit rate
        if let Some(latest_cache) = history.cache_hit_rate.back() {
            if latest_cache.value < 20.0 && history.cache_hit_rate.len() > 5 {
                return Some("Low cache hit rate".to_string());
            }
        }
        
        None
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

/// Calculate statistics for a data series
fn calculate_stats_for_data(data: &std::collections::VecDeque<crate::models::TimestampedValue>) -> crate::models::MetricStats {
    let values: Vec<f64> = data.iter().map(|tv| tv.value).collect();
    
    if values.is_empty() {
        return crate::models::MetricStats {
            mean: 0.0,
            min: 0.0,
            max: 0.0,
            std_dev: 0.0,
            count: 0,
        };
    }
    
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    
    let variance = values.iter()
        .map(|v| (v - mean).powi(2))
        .sum::<f64>() / values.len() as f64;
    let std_dev = variance.sqrt();
    
    crate::models::MetricStats {
        mean,
        min,
        max,
        std_dev,
        count: values.len(),
    }
}

/// Build an error menu for display when things go wrong
pub fn build_error_menu(message: &str) -> Result<String, std::fmt::Error> {
    let mut error_item = ContentItem::new(message);
    error_item = error_item.color("#ff0000").unwrap();
    error_item = error_item.font("Menlo").size(11);
    
    let mut retry_item = ContentItem::new("🔄 Retry");
    retry_item = retry_item.refresh();
    
    let menu = Menu(vec![
        MenuItem::Content(ContentItem::new("⚠️ Plugin Error")),
        MenuItem::Sep,
        MenuItem::Content(error_item),
        MenuItem::Sep,
        MenuItem::Content(retry_item),
    ]);
    Ok(menu.to_string())
}

/// Build a minimal menu for when service is not installed
pub fn build_not_installed_menu() -> String {
    let mut service_msg = ContentItem::new("Service not installed");
    service_msg = service_msg.color("#666666").unwrap();
    
    let mut doc_link = ContentItem::new("Visit documentation...");
    doc_link = doc_link.href("https://github.com/your-org/llama-swap").unwrap();
    
    let menu = Menu(vec![
        MenuItem::Content(ContentItem::new("⚪ Llama-Swap")),
        MenuItem::Sep,
        MenuItem::Content(service_msg),
        MenuItem::Content(doc_link),
    ]);
    menu.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PluginState;
    use crate::models::{MetricsHistory, TimestampedValue};
    use std::time::{SystemTime, UNIX_EPOCH};
    
    #[test]
    fn test_menu_with_running_service() {
        let state = create_test_state(ServiceStatus::Running);
        let menu_str = build_menu(&state).unwrap();
        
        // Should contain stop option
        assert!(menu_str.contains("Stop Service"));
        assert!(!menu_str.contains("Start Service"));
    }
    
    #[test]
    fn test_menu_with_stopped_service() {
        let state = create_test_state(ServiceStatus::Stopped);
        let menu_str = build_menu(&state).unwrap();
        
        // Should contain start option
        assert!(menu_str.contains("Start Service"));
        assert!(!menu_str.contains("Stop Service"));
        
        // Should not show metrics
        assert!(!menu_str.contains("Performance Metrics"));
        assert!(!menu_str.contains("Generation Speed"));
    }
    
    #[test]
    fn test_anomaly_detection() {
        let mut state = create_test_state(ServiceStatus::Running);
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        
        // Add normal memory values
        for i in 0..10 {
            state.metrics_history.memory_mb.push_back(TimestampedValue {
                timestamp: now - (10 - i) * 60,
                value: 1024.0,
            });
        }
        
        // Add spike
        state.metrics_history.memory_mb.push_back(TimestampedValue {
            timestamp: now,
            value: 5120.0, // 5GB - over threshold
        });
        
        let menu_str = build_menu(&state).unwrap();
        
        // Should show high memory warning
        assert!(menu_str.contains("High memory usage"));
    }
    
    #[test]
    fn test_error_menu() {
        let error_menu = build_error_menu("Test error message").unwrap();
        
        assert!(error_menu.contains("Plugin Error"));
        assert!(error_menu.contains("Test error message"));
        assert!(error_menu.contains("Retry"));
    }
    
    #[test]
    fn test_not_installed_menu() {
        let menu = build_not_installed_menu();
        
        assert!(menu.contains("Llama-Swap"));
        assert!(menu.contains("Service not installed"));
        assert!(menu.contains("Visit documentation"));
    }
    
    fn create_test_state(status: ServiceStatus) -> PluginState {
        PluginState {
            current_status: status,
            metrics_history: MetricsHistory::new(),
            error_count: 0,
        }
    }
}