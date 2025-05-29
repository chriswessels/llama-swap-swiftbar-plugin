use bitbar::{Menu, MenuItem, ContentItem, attr};
use crate::{icons, charts};
use crate::models::{ServiceStatus, AllModelMetricsHistory, AllModelMetrics, MetricsHistory, TimestampedValue, Trend};
use reqwest::blocking::Client;
use std::collections::VecDeque;

pub struct PluginState {
    #[allow(dead_code)]
    pub http_client: Client,
    pub metrics_history: AllModelMetricsHistory,
    pub current_status: ServiceStatus,
    pub current_all_metrics: Option<AllModelMetrics>,
    #[allow(dead_code)]
    pub error_count: usize,
}

#[derive(Clone)]
enum MetricDisplayType {
    Simple,
    SystemMemory,
    LlamaMemory,
    KvCache,
}

struct MenuBuilder {
    items: Vec<MenuItem>,
}

impl MenuBuilder {
    fn new() -> Self {
        Self { items: Vec::new() }
    }
    
    fn add_title(&mut self, status: ServiceStatus) {
        match icons::get_status_icon_png(status) {
            Ok(menu_image) => {
                let item = ContentItem::new("").image(menu_image).unwrap();
                self.items.push(MenuItem::Content(item));
            }
            Err(e) => {
                eprintln!("Failed to generate status icon: {}", e);
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
    
    fn add_header(&mut self, title: &str) {
        let mut header = ContentItem::new(title);
        header = header.color("#666666").unwrap();
        self.items.push(MenuItem::Content(header));
    }
    
    
    fn add_model_metrics_section(&mut self, model_name: &str, history: &MetricsHistory, current_metrics: &crate::models::Metrics) {
        self.add_header(model_name);
        
        if let Some(item) = self.create_metric("Generation", &history.tps, None, charts::MetricType::Tps, format_tps, MetricDisplayType::Simple, history) {
            self.items.push(item);
        }
        
        if let Some(item) = self.create_metric("Prompt", &history.prompt_tps, None, charts::MetricType::Prompt, format_tps, MetricDisplayType::Simple, history) {
            self.items.push(item);
        }
        
        if let Some(item) = self.create_metric("KV Cache", &history.kv_cache_percent, Some(&history.kv_cache_tokens), charts::MetricType::KvCache, format_percent, MetricDisplayType::KvCache, history) {
            self.items.push(item);
        }
        
        self.add_queue_status(current_metrics);
    }
    
    fn add_system_metrics_section(&mut self, history: &AllModelMetricsHistory) {
        self.add_header("System Metrics");
        
        if !history.cpu_usage_percent.is_empty() {
            if let Some(item) = self.create_system_metric("CPU", &history.cpu_usage_percent, None, charts::MetricType::Tps, format_percent, MetricDisplayType::Simple, history) {
                self.items.push(item);
            }
        }
        
        if !history.memory_usage_percent.is_empty() && !history.used_memory_gb.is_empty() {
            if let Some(item) = self.create_system_metric("Memory", &history.memory_usage_percent, Some(&history.used_memory_gb), charts::MetricType::Memory, format_percent, MetricDisplayType::SystemMemory, history) {
                self.items.push(item);
            }
        }
        
        if !history.total_llama_memory_mb.is_empty() && !history.used_memory_gb.is_empty() {
            if let Some(item) = self.create_system_metric("Llama Memory", &history.total_llama_memory_mb, Some(&history.used_memory_gb), charts::MetricType::Memory, format_memory, MetricDisplayType::LlamaMemory, history) {
                self.items.push(item);
            }
        }
    }
    
    fn create_metric(
        &self,
        name: &str,
        primary_data: &VecDeque<TimestampedValue>,
        secondary_data: Option<&VecDeque<TimestampedValue>>,
        chart_type: charts::MetricType,
        format_fn: fn(f64) -> String,
        display_type: MetricDisplayType,
        history: &MetricsHistory,
    ) -> Option<MenuItem> {
        if primary_data.is_empty() {
            return None;
        }
        
        let insights = history.get_insights(primary_data);
        let label = build_label(name, &insights, secondary_data, format_fn, &display_type);
        let mut item = ContentItem::new(label);
        
        add_chart(&mut item, primary_data, chart_type);
        let submenu = build_submenu(&insights, primary_data, secondary_data, format_fn, &display_type, Some(history), None);
        item = item.sub(submenu);
        
        Some(MenuItem::Content(item))
    }
    
    fn create_system_metric(
        &self,
        name: &str,
        primary_data: &VecDeque<TimestampedValue>,
        secondary_data: Option<&VecDeque<TimestampedValue>>,
        chart_type: charts::MetricType,
        format_fn: fn(f64) -> String,
        display_type: MetricDisplayType,
        history: &AllModelMetricsHistory,
    ) -> Option<MenuItem> {
        if primary_data.is_empty() {
            return None;
        }
        
        let insights = get_system_insights(name, history);
        let label = build_label(name, &insights, secondary_data, format_fn, &display_type);
        let mut item = ContentItem::new(label);
        
        add_chart(&mut item, primary_data, chart_type);
        let submenu = build_submenu(&insights, primary_data, secondary_data, format_fn, &display_type, None, Some(history));
        item = item.sub(submenu);
        
        Some(MenuItem::Content(item))
    }
    
    fn add_queue_status(&mut self, current_metrics: &crate::models::Metrics) {
        let queue_status = current_metrics.queue_status();
        let mut queue_item = ContentItem::new(format!("Queue: {}", queue_status));
        
        let color = if current_metrics.requests_processing > 0 || current_metrics.requests_deferred > 0 {
            "#FFA500"
        } else {
            "#666666"
        };
        queue_item = queue_item.color(color).unwrap();
        
        let submenu = vec![
            MenuItem::Content(ContentItem::new(format!("Status: {}", queue_status))),
            MenuItem::Content(ContentItem::new(format!("Processing: {} requests", current_metrics.requests_processing))),
            MenuItem::Content(ContentItem::new(format!("Deferred: {} requests", current_metrics.requests_deferred))),
            MenuItem::Content(ContentItem::new(format!("Decode Calls: {}", current_metrics.n_decode_total))),
        ];
        
        queue_item = queue_item.sub(submenu);
        self.items.push(MenuItem::Content(queue_item));
    }
    
    fn add_settings_section(&mut self, status: ServiceStatus, has_models: bool) {
        let exe = std::env::current_exe().unwrap();
        let exe_str = exe.to_str().unwrap();
        
        let mut submenu = Vec::new();
        
        // Control actions
        match status {
            ServiceStatus::Running => {
                let mut item = ContentItem::new("🔴 Stop Service");
                item = item.command(attr::Command::try_from((exe_str, "do_stop")).unwrap()).unwrap();
                submenu.push(MenuItem::Content(item));
                
                if has_models {
                    let mut unload = ContentItem::new("🗑️ Unload Model(s)");
                    unload = unload.command(attr::Command::try_from((exe_str, "do_unload")).unwrap()).unwrap();
                    submenu.push(MenuItem::Content(unload));
                }
            }
            ServiceStatus::Stopped | ServiceStatus::Unknown => {
                let mut item = ContentItem::new("🟢 Start Service");
                item = item.command(attr::Command::try_from((exe_str, "do_start")).unwrap()).unwrap();
                submenu.push(MenuItem::Content(item));
            }
        }
        
        let mut restart = ContentItem::new("⟲ Restart Service");
        restart = restart.command(attr::Command::try_from((exe_str, "do_restart")).unwrap()).unwrap();
        submenu.push(MenuItem::Content(restart));
        
        submenu.push(MenuItem::Sep);
        
        // File actions
        let mut view_logs = ContentItem::new("📄 View Logs");
        view_logs = view_logs.command(attr::Command::try_from((exe_str, "view_logs")).unwrap()).unwrap();
        submenu.push(MenuItem::Content(view_logs));
        
        let mut edit_config = ContentItem::new("⚙️ Edit Model Configuration");
        edit_config = edit_config.command(attr::Command::try_from((exe_str, "view_config")).unwrap()).unwrap();
        submenu.push(MenuItem::Content(edit_config));
        
        // Debug actions
        if cfg!(debug_assertions) {
            submenu.push(MenuItem::Sep);
            let mut refresh_item = ContentItem::new("🔄 Force Refresh");
            refresh_item = refresh_item.refresh();
            submenu.push(MenuItem::Content(refresh_item));
        }
        
        let mut settings_item = ContentItem::new("⚙️ Settings");
        settings_item = settings_item.sub(submenu);
        self.items.push(MenuItem::Content(settings_item));
    }
    
    fn build(self) -> Menu {
        Menu(self.items)
    }
}

fn build_label(
    name: &str,
    insights: &crate::models::MetricInsights,
    secondary_data: Option<&VecDeque<TimestampedValue>>,
    format_fn: fn(f64) -> String,
    display_type: &MetricDisplayType,
) -> String {
    match display_type {
        MetricDisplayType::Simple => format!("{}: {}", name, format_fn(insights.current)),
        MetricDisplayType::SystemMemory => {
            let gb_current = secondary_data.unwrap().back().unwrap().value;
            format!("{}: {:.1} GB ({:.1}%)", name, gb_current, insights.current)
        },
        MetricDisplayType::LlamaMemory => {
            let mb_current = insights.current;
            if mb_current < 1024.0 {
                format!("{}: {:.1} MB", name, mb_current)
            } else {
                format!("{}: {:.2} GB", name, mb_current / 1024.0)
            }
        },
        MetricDisplayType::KvCache => {
            let tokens_current = secondary_data.unwrap().back().unwrap().value as u32;
            format!("{}: {} tokens ({:.0}%)", name, tokens_current, insights.current)
        },
    }
}

fn add_chart(item: &mut ContentItem, data: &VecDeque<TimestampedValue>, chart_type: charts::MetricType) {
    let values = data.iter().map(|tv| tv.value).collect();
    if let Ok(chart) = charts::generate_sparkline(&values, chart_type) {
        if let Ok(chart_image) = icons::chart_to_menu_image(chart) {
            // We need to replace the item content, not clone it
            let text = item.text.clone();
            *item = ContentItem::new(text).image(chart_image).unwrap();
        }
    }
}

fn get_system_insights(metric_name: &str, history: &AllModelMetricsHistory) -> crate::models::MetricInsights {
    match metric_name {
        "CPU" => history.get_cpu_insights(),
        "Memory" => history.get_system_memory_insights(),
        "Llama Memory" => history.get_memory_insights(),
        _ => unreachable!(),
    }
}

fn build_submenu(
    insights: &crate::models::MetricInsights,
    primary_data: &VecDeque<TimestampedValue>,
    secondary_data: Option<&VecDeque<TimestampedValue>>,
    format_fn: fn(f64) -> String,
    display_type: &MetricDisplayType,
    model_history: Option<&MetricsHistory>,
    system_history: Option<&AllModelMetricsHistory>,
) -> Vec<MenuItem> {
    let mut submenu = Vec::new();
    
    // Current value
    let current_text = match display_type {
        MetricDisplayType::KvCache => {
            let tokens = secondary_data.unwrap().back().unwrap().value as u32;
            format!("Current: {} tokens ({:.0}%)", tokens, insights.current)
        },
        MetricDisplayType::SystemMemory => {
            let gb_current = secondary_data.unwrap().back().unwrap().value;
            format!("Current: {:.1} GB ({:.1}%)", gb_current, insights.current)
        },
        MetricDisplayType::LlamaMemory => {
            let mb_current = insights.current;
            let gb_current = mb_current / 1024.0;
            let total_system_memory_gb = calculate_total_system_memory(system_history.unwrap());
            let memory_percent = if total_system_memory_gb > 0.0 {
                (gb_current / total_system_memory_gb) * 100.0
            } else {
                0.0
            };
            
            if mb_current < 1024.0 {
                format!("Current: {:.1} MB ({:.1}% of system)", mb_current, memory_percent)
            } else {
                format!("Current: {:.2} GB ({:.1}% of system)", gb_current, memory_percent)
            }
        },
        _ => format!("Current: {}", format_fn(insights.current)),
    };
    submenu.push(MenuItem::Content(ContentItem::new(current_text)));
    
    // Range and statistics
    if insights.data_points > 1 {
        submenu.push(MenuItem::Content(
            ContentItem::new(format!("Range: {:.1} - {:.1}", insights.min, insights.max))
        ));
        
        match display_type {
            MetricDisplayType::KvCache => {
                let stats = model_history.unwrap().calculate_stats(secondary_data.unwrap());
                submenu.push(MenuItem::Content(
                    ContentItem::new(format!("Avg Tokens: {:.0}", stats.mean))
                ));
            },
            MetricDisplayType::SystemMemory => {
                if let Some(secondary) = secondary_data {
                    if secondary.len() > 1 {
                        let gb_values: Vec<f64> = secondary.iter().map(|tv| tv.value).collect();
                        let gb_min = gb_values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                        let gb_max = gb_values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                        submenu.push(MenuItem::Content(
                            ContentItem::new(format!("GB Range: {:.1} - {:.1}", gb_min, gb_max))
                        ));
                    }
                }
            },
            MetricDisplayType::LlamaMemory => {
                let stats = system_history.unwrap().calculate_memory_stats();
                submenu.push(MenuItem::Content(
                    ContentItem::new(format!("Average: {}", format_memory(stats.mean)))
                ));
                
                let total_system_memory_gb = calculate_total_system_memory(system_history.unwrap());
                submenu.push(MenuItem::Content(
                    ContentItem::new(format!("System Total: {:.1} GB", total_system_memory_gb))
                ));
            },
            _ => {
                let stats = calculate_stats_for_data(primary_data);
                submenu.push(MenuItem::Content(
                    ContentItem::new(format!("Average: {}", format_fn(stats.mean)))
                ));
            },
        }
    }
    
    // Trend information
    if insights.data_points >= 3 {
        let trend_desc = match insights.trend {
            Trend::Increasing => "▲ Increasing",
            Trend::Decreasing => "▼ Decreasing",
            Trend::Stable => "▶ Stable",
            Trend::Insufficient => "Insufficient data",
        };
        submenu.push(MenuItem::Content(
            ContentItem::new(format!("Trend: {}", trend_desc)).color(insights.trend.color()).unwrap()
        ));
    }
    
    // Dataset duration
    let time_text = if primary_data.len() >= 2 {
        let oldest = primary_data.front().unwrap().timestamp;
        let newest = primary_data.back().unwrap().timestamp;
        insights.time_context(oldest, newest)
    } else if primary_data.len() == 1 {
        insights.time_context(0, 0)
    } else {
        String::new()
    };
    
    if !time_text.is_empty() {
        submenu.push(MenuItem::Content(
            ContentItem::new(format!("Dataset: {}", time_text))
        ));
    }
    
    submenu
}

fn calculate_total_system_memory(history: &AllModelMetricsHistory) -> f64 {
    if let (Some(latest_used_gb), Some(latest_percent)) = (
        history.used_memory_gb.back(),
        history.memory_usage_percent.back()
    ) {
        let used_memory_gb = latest_used_gb.value;
        let memory_percent = latest_percent.value;
        if memory_percent > 0.0 {
            used_memory_gb / (memory_percent / 100.0)
        } else {
            64.0
        }
    } else {
        64.0
    }
}

fn format_memory(mb: f64) -> String {
    if mb < 1024.0 {
        format!("{:.1} MB", mb)
    } else {
        format!("{:.2} GB", mb / 1024.0)
    }
}

fn format_tps(v: f64) -> String {
    format!("{:.1} tok/s", v)
}

fn format_percent(v: f64) -> String {
    format!("{:.1}%", v)
}

fn calculate_stats_for_data(data: &VecDeque<TimestampedValue>) -> crate::models::MetricStats {
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

pub fn build_menu(state: &PluginState) -> crate::Result<String> {
    let mut menu = MenuBuilder::new();
    
    menu.add_title(state.current_status);
    menu.add_separator();
    let has_models = state.current_all_metrics
        .as_ref()
        .map(|m| !m.models.is_empty())
        .unwrap_or(false);
    
    if state.current_status == ServiceStatus::Running {
        menu.add_separator();
        menu.add_system_metrics_section(&state.metrics_history);
        
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
    }
    
    menu.add_separator();
    menu.add_settings_section(state.current_status, has_models);
    
    let built_menu = menu.build();
    Ok(built_menu.to_string())
}

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


#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_menu_with_running_service() {
        let state = create_test_state(ServiceStatus::Running);
        let menu_str = build_menu(&state).unwrap();
        
        assert!(menu_str.contains("Stop Service"));
        assert!(!menu_str.contains("Start Service"));
    }
    
    #[test]
    fn test_menu_with_stopped_service() {
        let state = create_test_state(ServiceStatus::Stopped);
        let menu_str = build_menu(&state).unwrap();
        
        assert!(menu_str.contains("Start Service"));
        assert!(!menu_str.contains("Stop Service"));
    }
    
    #[test]
    fn test_error_menu() {
        let error_menu = build_error_menu("Test error message").unwrap();
        
        assert!(error_menu.contains("Plugin Error"));
        assert!(error_menu.contains("Test error message"));
        assert!(error_menu.contains("Retry"));
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