use bitbar::{Menu, MenuItem, ContentItem, attr};
use crate::{icons, charts, constants};
use crate::models::{ServiceStatus, AllModelMetricsHistory, AllModelMetrics, MetricsHistory};
use reqwest::blocking::Client;

// PluginState for menu display - mirrors main::PluginState
pub struct PluginState {
    pub http_client: Client,
    pub metrics_history: AllModelMetricsHistory,
    pub current_status: ServiceStatus,
    pub current_all_metrics: Option<AllModelMetrics>,
    pub error_count: usize,
}

/// Build the complete menu based on current state
pub fn build_menu(state: &PluginState) -> crate::Result<String> {
    let mut menu = MenuBuilder::new();
    
    // Add title with status icon
    menu.add_title(state.current_status);
    
    // Add sections based on state
    menu.add_separator();
    menu.add_control_section(state.current_status);
    
    menu.add_separator();
    menu.add_file_section();
    
    // Show metrics for each running model
    if state.current_status == ServiceStatus::Running {
        if let Some(ref all_metrics) = state.current_all_metrics {
            for model_metrics in &all_metrics.models {
                if let Some(model_history) = state.metrics_history.get_model_history(&model_metrics.model_name) {
                    if !model_history.tps.is_empty() {
                        menu.add_separator();
                        menu.add_model_metrics_section(&model_metrics.model_name, model_history, &model_metrics.metrics);
                    }
                }
            }
        }
        
        // Add system metrics section
        menu.add_separator();
        menu.add_system_metrics_section(&state.metrics_history);
        
        // Add llama memory section if we have any models
        if !state.metrics_history.total_llama_memory_mb.is_empty() {
            menu.add_separator();
            menu.add_llama_memory_section(&state.metrics_history);
        }
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
    
    fn add_model_metrics_section(&mut self, model_name: &str, history: &MetricsHistory, current_metrics: &crate::models::Metrics) {
        // Section header with model name
        let mut header = ContentItem::new(model_name);
        header = header.color("#666666").unwrap();
        self.items.push(MenuItem::Content(header));
        
        // Generation Speed with dropdown details
        if let Some(item) = self.create_metric_with_dropdown(
            "Generation",
            &history.tps,
            charts::generate_tps_sparkline,
            |v| format!("{:.1} tok/s", v),
            history,
        ) {
            self.items.push(item);
        }
        
        // Prompt Speed with dropdown details  
        if let Some(item) = self.create_metric_with_dropdown(
            "Prompt",
            &history.prompt_tps,
            charts::generate_prompt_sparkline,
            |v| format!("{:.1} tok/s", v),
            history,
        ) {
            self.items.push(item);
        }
        
        // KV Cache with dropdown details
        if let Some(item) = self.create_metric_with_dropdown(
            "KV Cache",
            &history.kv_cache_percent,
            charts::generate_kv_cache_sparkline,
            |v| format!("{:.0}%", v),
            history,
        ) {
            self.items.push(item);
        }
        
        // Queue Status (simple display, no sparkline needed)
        self.add_model_queue_status_item(current_metrics);
    }
    
    fn add_system_metrics_section(&mut self, history: &AllModelMetricsHistory) {
        // Section header
        let mut header = ContentItem::new("System Metrics");
        header = header.color("#666666").unwrap();
        self.items.push(MenuItem::Content(header));
        
        // CPU usage
        if !history.cpu_usage_percent.is_empty() {
            if let Some(item) = self.create_system_metric_with_dropdown(
                "CPU",
                &history.cpu_usage_percent,
                charts::generate_tps_sparkline, // Reuse TPS sparkline for percentages
                |v| format!("{:.1}%", v),
                history,
                "cpu",
            ) {
                self.items.push(item);
            }
        }
        
        // System memory usage (percentage)
        if !history.memory_usage_percent.is_empty() {
            if let Some(item) = self.create_system_metric_with_dropdown(
                "Memory",
                &history.memory_usage_percent,
                charts::generate_memory_sparkline,
                |v| format!("{:.1}%", v),
                history,
                "memory",
            ) {
                self.items.push(item);
            }
        }
        
        // Load average
        if !history.load_average_1m.is_empty() {
            if let Some(item) = self.create_system_metric_with_dropdown(
                "Load Avg",
                &history.load_average_1m,
                charts::generate_prompt_sparkline, // Reuse prompt sparkline
                |v| format!("{:.2}", v),
                history,
                "load",
            ) {
                self.items.push(item);
            }
        }
        
        // GPU metrics removed - powermetrics was too expensive and unreliable
    }
    
    fn add_llama_memory_section(&mut self, history: &AllModelMetricsHistory) {
        // Section header
        let mut header = ContentItem::new("Llama Memory");
        header = header.color("#666666").unwrap();
        self.items.push(MenuItem::Content(header));
        
        // Memory with dropdown details
        if let Some(item) = self.create_memory_metric_with_dropdown(
            "Memory",
            &history.total_llama_memory_mb,
            charts::generate_memory_sparkline,
            |v| format_memory(v),
            history,
        ) {
            self.items.push(item);
        }
    }

    /// Create a metric item with enhanced sparkline and detailed dropdown submenu
    fn create_metric_with_dropdown<F, G>(
        &self,
        name: &str,
        data: &std::collections::VecDeque<crate::models::TimestampedValue>,
        chart_fn: F,
        format_fn: G,
        history: &crate::models::MetricsHistory,
    ) -> Option<MenuItem>
    where
        F: Fn(&std::collections::VecDeque<f64>) -> crate::Result<image::DynamicImage>,
        G: Fn(f64) -> String,
    {
        if data.is_empty() {
            return None;
        }
        
        let values = data.iter().map(|tv| tv.value).collect();
        let insights = history.get_insights(data);
        
        // Build enhanced label with trend arrow and range context
        let mut label = format!("{}: {}", name, format_fn(insights.current));
        
        // Add trend arrow
        if insights.data_points >= 3 {
            label.push_str(&format!(" {}", insights.trend.as_arrow()));
        }
        
        // Add time context using actual timestamps
        let time_text = if data.len() >= 2 {
            let oldest = data.front().unwrap().timestamp;
            let newest = data.back().unwrap().timestamp;
            insights.time_context(oldest, newest)
        } else if data.len() == 1 {
            insights.time_context(0, 0) // Will return "(now)"
        } else {
            String::new()
        };
        
        if !time_text.is_empty() {
            label.push_str(&format!(" {}", time_text));
        }
        
        let mut item = ContentItem::new(label);
        
        // Apply trend color to the text
        if insights.data_points >= 3 {
            item = item.color(insights.trend.color()).unwrap();
        }
        
        // Add enhanced sparkline chart
        if let Ok(chart) = chart_fn(&values) {
            if let Ok(chart_image) = icons::icon_to_menu_image(chart) {
                item = item.image(chart_image).unwrap();
            }
        }
        
        // Create detailed submenu items
        let mut submenu_items = vec![];
        
        // Current value
        submenu_items.push(MenuItem::Content(
            ContentItem::new(format!("Current: {}", format_fn(insights.current)))
        ));
        
        // Range (if we have multiple data points)
        if insights.data_points > 1 {
            submenu_items.push(MenuItem::Content(
                ContentItem::new(format!("Range: {:.1} - {:.1}", insights.min, insights.max))
            ));
            
            // Calculate and show average
            let stats = calculate_stats_for_data(data);
            submenu_items.push(MenuItem::Content(
                ContentItem::new(format!("Average: {}", format_fn(stats.mean)))
            ));
        }
        
        // Trend information
        if insights.data_points >= 3 {
            let trend_desc = match insights.trend {
                crate::models::Trend::Increasing => "▲ Increasing",
                crate::models::Trend::Decreasing => "▼ Decreasing", 
                crate::models::Trend::Stable => "▶ Stable",
                crate::models::Trend::Insufficient => "Insufficient data",
            };
            submenu_items.push(MenuItem::Content(
                ContentItem::new(format!("Trend: {}", trend_desc)).color(insights.trend.color()).unwrap()
            ));
        }
        
        // Add submenu to main item
        item = item.sub(submenu_items);
        
        Some(MenuItem::Content(item))
    }
    
    /// Create a memory metric item with enhanced sparkline and detailed dropdown submenu
    fn create_memory_metric_with_dropdown<F, G>(
        &self,
        name: &str,
        data: &std::collections::VecDeque<crate::models::TimestampedValue>,
        chart_fn: F,
        format_fn: G,
        history: &AllModelMetricsHistory,
    ) -> Option<MenuItem>
    where
        F: Fn(&std::collections::VecDeque<f64>) -> crate::Result<image::DynamicImage>,
        G: Fn(f64) -> String,
    {
        if data.is_empty() {
            return None;
        }
        
        let values = data.iter().map(|tv| tv.value).collect();
        let insights = history.get_memory_insights();
        
        // Build enhanced label with trend arrow and range context
        let mut label = format!("{}: {}", name, format_fn(insights.current));
        
        // Add trend arrow
        if insights.data_points >= 3 {
            label.push_str(&format!(" {}", insights.trend.as_arrow()));
        }
        
        // Add time context using actual timestamps
        let time_text = if data.len() >= 2 {
            let oldest = data.front().unwrap().timestamp;
            let newest = data.back().unwrap().timestamp;
            insights.time_context(oldest, newest)
        } else if data.len() == 1 {
            insights.time_context(0, 0) // Will return "(now)"
        } else {
            String::new()
        };
        
        if !time_text.is_empty() {
            label.push_str(&format!(" {}", time_text));
        }
        
        let mut item = ContentItem::new(label);
        
        // Apply trend color to the text
        if insights.data_points >= 3 {
            item = item.color(insights.trend.color()).unwrap();
        }
        
        // Add enhanced sparkline chart
        if let Ok(chart) = chart_fn(&values) {
            if let Ok(chart_image) = icons::icon_to_menu_image(chart) {
                item = item.image(chart_image).unwrap();
            }
        }
        
        // Create detailed submenu items
        let mut submenu_items = vec![];
        
        // Current value
        submenu_items.push(MenuItem::Content(
            ContentItem::new(format!("Current: {}", format_fn(insights.current)))
        ));
        
        // Range (if we have multiple data points)
        if insights.data_points > 1 {
            submenu_items.push(MenuItem::Content(
                ContentItem::new(format!("Range: {:.1} - {:.1}", insights.min, insights.max))
            ));
            
            // Calculate and show average
            let stats = history.calculate_memory_stats();
            submenu_items.push(MenuItem::Content(
                ContentItem::new(format!("Average: {}", format_fn(stats.mean)))
            ));
        }
        
        // Trend information
        if insights.data_points >= 3 {
            let trend_desc = match insights.trend {
                crate::models::Trend::Increasing => "▲ Increasing",
                crate::models::Trend::Decreasing => "▼ Decreasing", 
                crate::models::Trend::Stable => "▶ Stable",
                crate::models::Trend::Insufficient => "Insufficient data",
            };
            submenu_items.push(MenuItem::Content(
                ContentItem::new(format!("Trend: {}", trend_desc)).color(insights.trend.color()).unwrap()
            ));
        }
        
        // Add submenu to main item
        item = item.sub(submenu_items);
        
        Some(MenuItem::Content(item))
    }
    
    /// Create a system metric item with enhanced sparkline and detailed dropdown submenu
    fn create_system_metric_with_dropdown<F, G>(
        &self,
        name: &str,
        data: &std::collections::VecDeque<crate::models::TimestampedValue>,
        chart_fn: F,
        format_fn: G,
        history: &AllModelMetricsHistory,
        metric_type: &str,
    ) -> Option<MenuItem>
    where
        F: Fn(&std::collections::VecDeque<f64>) -> crate::Result<image::DynamicImage>,
        G: Fn(f64) -> String,
    {
        if data.is_empty() {
            return None;
        }
        
        let values = data.iter().map(|tv| tv.value).collect();
        let insights = match metric_type {
            "cpu" => history.get_cpu_insights(),
            "memory" => history.get_system_memory_insights(),
            "load" => history.get_load_insights(),
            _ => return None,
        };
        
        // Build enhanced label with trend arrow and range context
        let mut label = format!("{}: {}", name, format_fn(insights.current));
        
        // Add trend arrow
        if insights.data_points >= 3 {
            label.push_str(&format!(" {}", insights.trend.as_arrow()));
        }
        
        // Add time context using actual timestamps
        let time_text = if data.len() >= 2 {
            let oldest = data.front().unwrap().timestamp;
            let newest = data.back().unwrap().timestamp;
            insights.time_context(oldest, newest)
        } else if data.len() == 1 {
            insights.time_context(0, 0) // Will return "(now)"
        } else {
            String::new()
        };
        
        if !time_text.is_empty() {
            label.push_str(&format!(" {}", time_text));
        }
        
        let mut item = ContentItem::new(label);
        
        // Apply trend color to the text
        if insights.data_points >= 3 {
            item = item.color(insights.trend.color()).unwrap();
        }
        
        // Add enhanced sparkline chart
        if let Ok(chart) = chart_fn(&values) {
            if let Ok(chart_image) = icons::icon_to_menu_image(chart) {
                item = item.image(chart_image).unwrap();
            }
        }
        
        // Create detailed submenu items
        let mut submenu_items = vec![];
        
        // Current value
        submenu_items.push(MenuItem::Content(
            ContentItem::new(format!("Current: {}", format_fn(insights.current)))
        ));
        
        // Range (if we have multiple data points)
        if insights.data_points > 1 {
            submenu_items.push(MenuItem::Content(
                ContentItem::new(format!("Range: {:.1} - {:.1}", insights.min, insights.max))
            ));
        }
        
        // Trend information
        if insights.data_points >= 3 {
            let trend_desc = match insights.trend {
                crate::models::Trend::Increasing => "▲ Increasing",
                crate::models::Trend::Decreasing => "▼ Decreasing", 
                crate::models::Trend::Stable => "▶ Stable",
                crate::models::Trend::Insufficient => "Insufficient data",
            };
            submenu_items.push(MenuItem::Content(
                ContentItem::new(format!("Trend: {}", trend_desc)).color(insights.trend.color()).unwrap()
            ));
        }
        
        // Add submenu to main item
        item = item.sub(submenu_items);
        
        Some(MenuItem::Content(item))
    }
    
    /// Add queue status item for a specific model
    fn add_model_queue_status_item(&mut self, current_metrics: &crate::models::Metrics) {
        let queue_status = current_metrics.queue_status();
        
        let mut queue_item = ContentItem::new(format!("Queue: {}", queue_status));
        
        // Color based on queue load
        let color = if current_metrics.requests_processing > 0 || current_metrics.requests_deferred > 0 {
            "#FFA500" // Orange for active
        } else {
            "#666666" // Gray for idle
        };
        queue_item = queue_item.color(color).unwrap();
        
        // Create queue details submenu
        let mut submenu_items = vec![];
        
        submenu_items.push(MenuItem::Content(
            ContentItem::new(format!("Status: {}", queue_status))
        ));
        
        submenu_items.push(MenuItem::Content(
            ContentItem::new(format!("Processing: {} requests", current_metrics.requests_processing))
        ));
        
        submenu_items.push(MenuItem::Content(
            ContentItem::new(format!("Deferred: {} requests", current_metrics.requests_deferred))
        ));
        
        submenu_items.push(MenuItem::Content(
            ContentItem::new(format!("Decode Calls: {}", current_metrics.n_decode_total))
        ));
        
        // Calculate slot utilization if we have decode data
        if current_metrics.n_decode_total > 0 {
            let slot_util = "1.0 avg"; // Could calculate from available data
            submenu_items.push(MenuItem::Content(
                ContentItem::new(format!("Slot Utilization: {}", slot_util))
            ));
        }
        
        queue_item = queue_item.sub(submenu_items);
        self.items.push(MenuItem::Content(queue_item));
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
            http_client: reqwest::blocking::Client::new(),
            current_status: status,
            metrics_history: AllModelMetricsHistory::new(),
            current_all_metrics: None,
            error_count: 0,
        }
    }
}