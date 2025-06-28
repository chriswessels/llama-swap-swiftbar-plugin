use bitbar::{Menu, MenuItem, ContentItem};
use crate::{icons, charts};
use crate::models::{AllMetricsHistory, MetricsHistory, TimestampedValue};
use crate::state_machines::program::ProgramStates;
use std::collections::VecDeque;

// Use the shared PluginState
use crate::types::PluginState;

/// Helper function to create colored menu items
fn create_colored_item(text: &str, color: &str) -> ContentItem {
    ContentItem::new(text).color(color).unwrap()
}

/// Helper function to create command menu items
fn create_command_item(text: &str, exe_path: &str, action: &str) -> crate::Result<ContentItem> {
    let command = bitbar::attr::Command::try_from((exe_path, action))?;
    Ok(ContentItem::new(text).command(command)?)
}

/// Convert program state color names to hex codes
fn get_hex_color(color: &str) -> &'static str {
    match color {
        "blue" => "#007AFF",
        "green" => "#34C759", 
        "yellow" => "#FF9500",
        "grey" => "#8E8E93",
        "red" => "#FF3B30",
        _ => "#8E8E93", // default grey
    }
}

/// Menu command configuration
struct MenuCommand {
    icon: &'static str,
    label: &'static str,
    action: &'static str,
    states: &'static [ProgramStates],
}

/// Settings menu configuration
static CONTROL_COMMANDS: &[MenuCommand] = &[
    MenuCommand {
        icon: ":eject:",
        label: "Unload Model(s)",
        action: "do_unload",
        states: &[ProgramStates::ModelProcessingQueue, ProgramStates::ModelReady, ProgramStates::ServiceLoadedNoModel],
    },
    MenuCommand {
        icon: ":stop.fill:",
        label: "Stop Llama-Swap Service",
        action: "do_stop",
        states: &[ProgramStates::ModelProcessingQueue, ProgramStates::ModelReady, ProgramStates::ServiceLoadedNoModel, ProgramStates::AgentStarting],
    },
    MenuCommand {
        icon: ":play.fill:",
        label: "Start Llama-Swap Service",
        action: "do_start",
        states: &[ProgramStates::AgentNotLoaded, ProgramStates::ModelLoading, ProgramStates::AgentStarting],
    },
];

static FILE_COMMANDS: &[MenuCommand] = &[
    MenuCommand {
        icon: ":doc.text.magnifyingglass:",
        label: "View Llama-Swap Logs",
        action: "view_logs",
        states: &[], // Available in all states
    },
    MenuCommand {
        icon: ":gearshape:",
        label: "Edit Llama-Swap Configuration",
        action: "view_config",
        states: &[], // Available in all states
    },
];

static RESTART_COMMAND: MenuCommand = MenuCommand {
    icon: ":arrow.2.circlepath:",
    label: "Restart Llama-Swap Service",
    action: "do_restart",
    states: &[], // Available in all states
};

impl MenuCommand {
    fn is_available_for_state(&self, state: ProgramStates) -> bool {
        self.states.is_empty() || self.states.contains(&state)
    }
    
    fn create_item(&self, exe_path: &str) -> crate::Result<ContentItem> {
        let text = format!("{} {}", self.icon, self.label);
        create_command_item(&text, exe_path, self.action)
    }
}

#[derive(Clone)]
enum MetricDisplayType {
    Simple,
    SystemMemory,
    LlamaMemory,
    KvCache,
}

enum MetricHistory<'a> {
    Model(&'a MetricsHistory),
    System(&'a AllMetricsHistory, &'static str),
}

struct MetricConfig<'a> {
    name: &'a str,
    primary_data: &'a VecDeque<TimestampedValue>,
    secondary_data: Option<&'a VecDeque<TimestampedValue>>,
    chart_type: charts::MetricType,
    format_fn: fn(f64) -> String,
    display_type: MetricDisplayType,
    history: MetricHistory<'a>,
}

impl<'a> MetricHistory<'a> {
    fn get_stats(&self, primary_data: &VecDeque<TimestampedValue>) -> crate::models::MetricStats {
        match self {
            MetricHistory::Model(history) => history.get_stats(primary_data),
            MetricHistory::System(history, metric_name) => get_system_stats(metric_name, history),
        }
    }
    
    fn build_submenu(
        &self,
        insights: &crate::models::MetricStats,
        primary_data: &VecDeque<TimestampedValue>,
        secondary_data: Option<&VecDeque<TimestampedValue>>,
        format_fn: fn(f64) -> String,
        display_type: &MetricDisplayType,
    ) -> Vec<MenuItem> {
        match self {
            MetricHistory::Model(history) => build_submenu(insights, primary_data, secondary_data, format_fn, display_type, Some(history), None),
            MetricHistory::System(history, _) => build_submenu(insights, primary_data, secondary_data, format_fn, display_type, None, Some(history)),
        }
    }
}

struct MenuBuilder {
    items: Vec<MenuItem>,
}

impl MenuBuilder {
    fn new() -> Self {
        Self { items: Vec::new() }
    }
    
    fn add_title(&mut self, program_state: ProgramStates) {
        let icon = icons::get_program_state_icon(program_state);
        let item = ContentItem::new("").image(icon.clone()).unwrap();
        self.items.push(MenuItem::Content(item));
    }
    
    fn add_status_message(&mut self, program_state: ProgramStates) {
        let message = program_state.status_message();
        let color = get_hex_color(program_state.icon_color());
        let status_item = create_colored_item(message, color);
        self.items.push(MenuItem::Content(status_item));
    }

    fn add_separator(&mut self) {
        self.items.push(MenuItem::Sep);
    }
    
    fn add_header(&mut self, title: &str) {
        let header = create_colored_item(title, "#666666");
        self.items.push(MenuItem::Content(header));
    }
    
    
    fn add_model_metrics_section(&mut self, model_name: &str, history: &MetricsHistory, current_metrics: &crate::models::Metrics) {
        self.add_header(model_name);
        
        if let Some(item) = self.create_metric(MetricConfig {
            name: "Prompt Processing",
            primary_data: &history.prompt_tps,
            secondary_data: None,
            chart_type: charts::MetricType::Prompt,
            format_fn: format_tps,
            display_type: MetricDisplayType::Simple,
            history: MetricHistory::Model(history),
        }) {
            self.items.push(item);
        }

        if let Some(item) = self.create_metric(MetricConfig {
            name: "Generation",
            primary_data: &history.tps,
            secondary_data: None,
            chart_type: charts::MetricType::Tps,
            format_fn: format_tps,
            display_type: MetricDisplayType::Simple,
            history: MetricHistory::Model(history),
        }) {
            self.items.push(item);
        }
        
        if let Some(item) = self.create_metric(MetricConfig {
            name: "KV Cache",
            primary_data: &history.kv_cache_percent,
            secondary_data: Some(&history.kv_cache_tokens),
            chart_type: charts::MetricType::KvCache,
            format_fn: format_percent,
            display_type: MetricDisplayType::KvCache,
            history: MetricHistory::Model(history),
        }) {
            self.items.push(item);
        }
        
        self.add_queue_status(current_metrics);
    }
    
    fn add_system_metrics_section(&mut self, history: &AllMetricsHistory) {
        self.add_header("System Metrics");
        
        if !history.cpu_usage_percent.is_empty() {
            if let Some(item) = self.create_metric(MetricConfig {
                name: "CPU",
                primary_data: &history.cpu_usage_percent,
                secondary_data: None,
                chart_type: charts::MetricType::Tps,
                format_fn: format_percent,
                display_type: MetricDisplayType::Simple,
                history: MetricHistory::System(history, "CPU"),
            }) {
                self.items.push(item);
            }
        }
        
        if !history.memory_usage_percent.is_empty() && !history.used_memory_gb.is_empty() {
            if let Some(item) = self.create_metric(MetricConfig {
                name: "Memory",
                primary_data: &history.memory_usage_percent,
                secondary_data: Some(&history.used_memory_gb),
                chart_type: charts::MetricType::Memory,
                format_fn: format_percent,
                display_type: MetricDisplayType::SystemMemory,
                history: MetricHistory::System(history, "Memory"),
            }) {
                self.items.push(item);
            }
        }
        
        if !history.total_llama_memory_mb.is_empty() && !history.used_memory_gb.is_empty() {
            if let Some(item) = self.create_metric(MetricConfig {
                name: "Llama Memory",
                primary_data: &history.total_llama_memory_mb,
                secondary_data: Some(&history.used_memory_gb),
                chart_type: charts::MetricType::Memory,
                format_fn: format_memory,
                display_type: MetricDisplayType::LlamaMemory,
                history: MetricHistory::System(history, "Llama Memory"),
            }) {
                self.items.push(item);
            }
        }
    }
    
    fn create_metric(&self, config: MetricConfig) -> Option<MenuItem> {
        if config.primary_data.is_empty() {
            return None;
        }
        
        let insights = config.history.get_stats(config.primary_data);
        let label = build_label(config.name, &insights, config.secondary_data, config.format_fn, &config.display_type);
        let mut item = ContentItem::new(label);
        
        add_chart(&mut item, config.primary_data, config.chart_type);
        let submenu = config.history.build_submenu(&insights, config.primary_data, config.secondary_data, config.format_fn, &config.display_type);
        item = item.sub(submenu);
        
        Some(MenuItem::Content(item))
    }
    
    fn add_queue_status(&mut self, current_metrics: &crate::models::Metrics) {
        let queue_status = current_metrics.queue_status();
        let color = if current_metrics.requests_processing > 0 || current_metrics.requests_deferred > 0 {
            "#FFA500"
        } else {
            "#666666"
        };
        
        let queue_item = create_colored_item(&format!("Queue: {queue_status}"), color)
            .sub(vec![
                MenuItem::Content(ContentItem::new(format!("Status: {queue_status}"))),
                MenuItem::Content(ContentItem::new(format!("Processing: {} requests", current_metrics.requests_processing))),
                MenuItem::Content(ContentItem::new(format!("Deferred: {} requests", current_metrics.requests_deferred))),
                MenuItem::Content(ContentItem::new(format!("Decode Calls: {}", current_metrics.n_decode_total))),
            ]);
        
        self.items.push(MenuItem::Content(queue_item));
    }
    
    
    fn add_settings_section(&mut self, program_state: ProgramStates, has_models: bool, state: &PluginState) {
        let exe = std::env::current_exe().unwrap();
        let exe_str = exe.to_str().unwrap();
        
        let mut submenu = Vec::new();
        
        // Add control commands based on current state
        for command in CONTROL_COMMANDS {
            if command.is_available_for_state(program_state) {
                // Special handling for unload command - only show if models are present
                if command.action == "do_unload" && !has_models {
                    continue;
                }
                
                if let Ok(item) = command.create_item(exe_str) {
                    submenu.push(MenuItem::Content(item));
                }
            }
        }
        
        // Always add restart command
        if let Ok(item) = RESTART_COMMAND.create_item(exe_str) {
            submenu.push(MenuItem::Content(item));
        }
        
        submenu.push(MenuItem::Sep);
        
        // Add file action commands
        for command in FILE_COMMANDS {
            if let Ok(item) = command.create_item(exe_str) {
                submenu.push(MenuItem::Content(item));
            }
        }
        
        submenu.push(MenuItem::Sep);
        submenu.push(MenuItem::Content(create_colored_item("Llama-Swap Swiftbar Plugin", "#666666")));
        
        // Debug actions - always available
        let refresh_item = ContentItem::new(":arrow.clockwise: Force Plugin Refresh").refresh();
        submenu.push(MenuItem::Content(refresh_item));
        
        // Add debug state info after refresh item
        let mut debug_submenu = Vec::new();
        
        // Agent state machine
        debug_submenu.push(MenuItem::Content(
            ContentItem::new(format!("Agent: {:?}", state.agent_state_machine.state()))
        ));
        
        // Program state machine
        debug_submenu.push(MenuItem::Content(
            ContentItem::new(format!("Program: {:?}", state.program_state_machine.state()))
        ));
        
        // Polling mode state machine
        debug_submenu.push(MenuItem::Content(
            ContentItem::new(format!("Polling: {:?}", state.polling_mode_state_machine.state()))
        ));
        
        // Model state machines
        if !state.model_state_machines.is_empty() {
            debug_submenu.push(MenuItem::Sep);
            debug_submenu.push(MenuItem::Content(
                ContentItem::new("Model States:")
            ));
            
            for (model_name, state_machine) in &state.model_state_machines {
                debug_submenu.push(MenuItem::Content(
                    ContentItem::new(format!("  {}: {:?}", model_name, state_machine.state()))
                ));
            }
        }
        
        // Error count and metrics info
        debug_submenu.push(MenuItem::Sep);
        debug_submenu.push(MenuItem::Content(
            ContentItem::new(format!("Error Count: {}", state.error_count))
        ));
        
        debug_submenu.push(MenuItem::Content(
            ContentItem::new(format!("Has Metrics: {}", state.current_all_metrics.is_some()))
        ));
        
        if let Some(ref metrics) = state.current_all_metrics {
            debug_submenu.push(MenuItem::Content(
                ContentItem::new(format!("Models Loaded: {}", metrics.models.len()))
            ));
        }
        
        let debug_item = ContentItem::new(":ladybug: Debug State Info")
            .sub(debug_submenu);
        submenu.push(MenuItem::Content(debug_item));
        
        let mut settings_item = ContentItem::new(":gearshape.fill: Advanced");
        settings_item = settings_item.sub(submenu);
        self.items.push(MenuItem::Content(settings_item));
    }
    
    fn build(self) -> Menu {
        Menu(self.items)
    }
}

fn build_label(
    name: &str,
    insights: &crate::models::MetricStats,
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
                format!("{name}: {mb_current:.1} MB")
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

fn get_system_stats(metric_name: &str, history: &AllMetricsHistory) -> crate::models::MetricStats {
    match metric_name {
        "CPU" => history.get_cpu_stats(),
        "Memory" => history.get_system_memory_stats(),
        "Llama Memory" => history.get_memory_stats(),
        _ => unreachable!(),
    }
}

fn build_submenu(
    insights: &crate::models::MetricStats,
    primary_data: &VecDeque<TimestampedValue>,
    secondary_data: Option<&VecDeque<TimestampedValue>>,
    format_fn: fn(f64) -> String,
    display_type: &MetricDisplayType,
    model_history: Option<&MetricsHistory>,
    system_history: Option<&AllMetricsHistory>,
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
                format!("Current: {mb_current:.1} MB ({memory_percent:.1}% of system)")
            } else {
                format!("Current: {gb_current:.2} GB ({memory_percent:.1}% of system)")
            }
        },
        _ => format!("Current: {}", format_fn(insights.current)),
    };
    submenu.push(MenuItem::Content(ContentItem::new(current_text)));
    
    // Range and statistics
    if insights.count > 1 {
        submenu.push(MenuItem::Content(
            ContentItem::new(format!("Range: {:.1} - {:.1}", insights.min, insights.max))
        ));
        
        match display_type {
            MetricDisplayType::KvCache => {
                let stats = model_history.unwrap().get_stats(secondary_data.unwrap());
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
                            ContentItem::new(format!("GB Range: {gb_min:.1} - {gb_max:.1}"))
                        ));
                    }
                }
            },
            MetricDisplayType::LlamaMemory => {
                let stats = system_history.unwrap().get_memory_stats();
                submenu.push(MenuItem::Content(
                    ContentItem::new(format!("Average: {}", format_memory(stats.mean)))
                ));
                
                let total_system_memory_gb = calculate_total_system_memory(system_history.unwrap());
                submenu.push(MenuItem::Content(
                    ContentItem::new(format!("System Total: {total_system_memory_gb:.1} GB"))
                ));
            },
            _ => {
                let stats = crate::models::DataAnalyzer::get_stats(primary_data);
                submenu.push(MenuItem::Content(
                    ContentItem::new(format!("Average: {}", format_fn(stats.mean)))
                ));
            },
        }
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
            ContentItem::new(format!("Dataset: {time_text}"))
        ));
    }
    
    submenu
}

fn calculate_total_system_memory(history: &AllMetricsHistory) -> f64 {
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
        format!("{mb:.1} MB")
    } else {
        format!("{:.2} GB", mb / 1024.0)
    }
}

fn format_tps(v: f64) -> String {
    format!("{v:.1} tok/s")
}

fn format_percent(v: f64) -> String {
    format!("{v:.1}%")
}


pub fn build_menu(state: &PluginState) -> crate::Result<String> {
    let mut menu = MenuBuilder::new();
    
    let current_program_state = *state.program_state_machine.state();
    
    menu.add_title(current_program_state);
    menu.add_separator();
    menu.add_status_message(current_program_state);
    menu.add_separator();
    
    let has_models = state.current_all_metrics
        .as_ref()
        .map(|m| !m.models.is_empty())
        .unwrap_or(false);
    
    if matches!(current_program_state, 
        ProgramStates::ModelProcessingQueue | 
        ProgramStates::ModelReady | 
        ProgramStates::ServiceLoadedNoModel) {
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
    menu.add_settings_section(current_program_state, has_models, state);
    
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
    use crate::state_machines::agent::AgentStates;
    
    #[test]
    fn test_menu_with_running_service() {
        let state = create_test_state_for_running_service();
        
        // Verify the state machines are in the expected states
        assert!(matches!(state.agent_state_machine.state(), AgentStates::Running));
        assert!(matches!(state.program_state_machine.state(), ProgramStates::ModelReady));
        
        let menu_str = build_menu(&state).unwrap();
        
        // When service is running and models are ready, should show stop options
        assert!(menu_str.contains("Stop Llama-Swap Service"));
        assert!(!menu_str.contains("Start Llama-Swap Service"));
    }
    
    #[test]
    fn test_menu_with_stopped_service() {
        let state = create_test_state_for_stopped_service();
        
        // Verify the state machines are in the expected states
        assert!(matches!(state.agent_state_machine.state(), AgentStates::NotInstalled));
        assert!(matches!(state.program_state_machine.state(), ProgramStates::AgentNotLoaded));
        
        let menu_str = build_menu(&state).unwrap();
        
        // When service is stopped, should show start options
        assert!(menu_str.contains("Start Llama-Swap Service"));
        assert!(!menu_str.contains("Stop Llama-Swap Service"));
    }
    
    #[test]
    fn test_error_menu() {
        let error_menu = build_error_menu("Test error message").unwrap();
        
        assert!(error_menu.contains("Plugin Error"));
        assert!(error_menu.contains("Test error message"));
        assert!(error_menu.contains("Retry"));
    }
    
    
    fn create_test_state_for_running_service() -> PluginState {
        use crate::state_machines::agent::{AgentEvents, ServiceRunning};
        use crate::state_machines::program::{ProgramEvents, AgentUpdate, ModelUpdate};
        use crate::models::{AllMetrics, ModelMetrics, ModelState, Metrics};
        
        let mut state = PluginState::new().unwrap();
        
        // Step 1: Drive agent state machine to Running state
        // Simulate service being detected as running
        let _ = state.agent_state_machine.process_event(AgentEvents::ServiceDetected(ServiceRunning(true)));
        // Simulate startup timeout to transition to Running
        let _ = state.agent_state_machine.process_event(AgentEvents::StartupComplete(crate::state_machines::agent::StartupTimeout));
        
        // Step 2: Drive program state machine by feeding it the running agent state
        let agent_state = *state.agent_state_machine.state();
        let _ = state.program_state_machine.process_event(ProgramEvents::AgentStateChanged(AgentUpdate(agent_state)));
        
        // Step 3: Simulate having models loaded and ready (ModelReady state)
        let model_update = ModelUpdate {
            has_models: true,
            has_loading: false,
            has_activity: false,
        };
        let _ = state.program_state_machine.process_event(ProgramEvents::ModelStateChanged(model_update));
        
        // Step 4: Set up some dummy metrics to make the state consistent
        let dummy_metrics = AllMetrics {
            models: vec![ModelMetrics {
                model_name: "test-model".to_string(),
                model_state: ModelState::Running,
                metrics: Metrics {
                    prompt_tokens_per_sec: 10.0,
                    predicted_tokens_per_sec: 15.0,
                    requests_processing: 0,
                    requests_deferred: 0,
                    kv_cache_usage_ratio: 0.5,
                    kv_cache_tokens: 256,
                    n_decode_total: 100,
                    memory_mb: 1000.0,
                },
            }],
            total_llama_memory_mb: 1000.0,
            system_metrics: crate::models::SystemMetrics {
                cpu_usage_percent: 25.0,
                total_memory_gb: 16.0,
                used_memory_gb: 8.0,
                available_memory_gb: 8.0,
                memory_usage_percent: 50.0,
            },
        };
        state.current_all_metrics = Some(dummy_metrics);
        
        state
    }
    
    fn create_test_state_for_stopped_service() -> PluginState {
        use crate::state_machines::agent::{AgentEvents, ServiceRunning};
        use crate::state_machines::program::{ProgramEvents, AgentUpdate, ModelUpdate};
        
        let mut state = PluginState::new().unwrap();
        
        // Step 1: Keep agent in stopped/not-installed state by not sending service running events
        // The agent starts in NotInstalled state, and without ServiceDetected(true), it stays there
        let _ = state.agent_state_machine.process_event(AgentEvents::ServiceDetected(ServiceRunning(false)));
        
        // Step 2: Drive program state machine with the stopped agent state
        let agent_state = *state.agent_state_machine.state();
        let _ = state.program_state_machine.process_event(ProgramEvents::AgentStateChanged(AgentUpdate(agent_state)));
        
        // Step 3: No models since service is not running
        let model_update = ModelUpdate {
            has_models: false,
            has_loading: false,
            has_activity: false,
        };
        let _ = state.program_state_machine.process_event(ProgramEvents::ModelStateChanged(model_update));
        
        // No metrics since service is stopped
        state.current_all_metrics = None;
        
        state
    }
}