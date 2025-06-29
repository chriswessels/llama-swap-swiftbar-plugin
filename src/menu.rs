use crate::models::{AllMetricsHistory, MetricsHistory, TimestampedValue};
use crate::state_model::DisplayState;
use crate::{charts, icons};
use bitbar::{ContentItem, Menu, MenuItem};
use circular_queue::CircularQueue;

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
        "red" => "#FF3B30",    // Problems/action required
        "grey" => "#8E8E93",   // Idle/neutral
        "yellow" => "#FF9500", // Transitional/loading
        "green" => "#34C759",  // Ready with models
        "blue" => "#007AFF",   // Active processing
        _ => "#8E8E93",        // default grey
    }
}

/// Menu command configuration
struct MenuCommand {
    icon: &'static str,
    label: &'static str,
    action: &'static str,
    states: &'static [DisplayState],
}

/// Settings menu configuration
static CONTROL_COMMANDS: &[MenuCommand] = &[
    MenuCommand {
        icon: ":eject:",
        label: "Unload Model(s)",
        action: "do_unload",
        states: &[
            DisplayState::ModelProcessingQueue,
            DisplayState::ModelReady,
            DisplayState::ServiceLoadedNoModel,
        ],
    },
    MenuCommand {
        icon: ":stop.fill:",
        label: "Stop Llama-Swap Service",
        action: "do_stop",
        states: &[
            DisplayState::ModelProcessingQueue,
            DisplayState::ModelReady,
            DisplayState::ServiceLoadedNoModel,
            DisplayState::AgentStarting,
        ],
    },
    MenuCommand {
        icon: ":play.fill:",
        label: "Start Llama-Swap Service",
        action: "do_start",
        states: &[DisplayState::ServiceStopped], // Fix: Available when stopped (ready to start)
    },
];

static FILE_COMMANDS: &[MenuCommand] = &[
    MenuCommand {
        icon: ":gearshape:",
        label: "Edit Llama-Swap Configuration",
        action: "view_config",
        states: &[], // Available in all states
    },
];
static UI_COMMAND: MenuCommand = MenuCommand {
    icon: ":globe:",
    label: "Open Llama-Swap UI",
    action: "open_ui",
    states: &[
        DisplayState::ModelProcessingQueue,
        DisplayState::ModelReady,
        DisplayState::ServiceLoadedNoModel,
    ], // Only when API is responsive
};

static RESTART_COMMAND: MenuCommand = MenuCommand {
    icon: ":arrow.2.circlepath:",
    label: "Restart Llama-Swap Service",
    action: "do_restart",
    states: &[], // Available in all states
};

static INSTALL_COMMAND: MenuCommand = MenuCommand {
    icon: ":arrow.down.doc:",
    label: "Install Llama-Swap Service",
    action: "do_install",
    states: &[DisplayState::AgentNotLoaded], // Only when not installed
};

static UNINSTALL_COMMAND: MenuCommand = MenuCommand {
    icon: ":trash:",
    label: "Uninstall Llama-Swap Service",
    action: "do_uninstall",
    states: &[], // Available when installed (all states except AgentNotLoaded)
};

impl MenuCommand {
    fn is_available_for_state(&self, state: DisplayState) -> bool {
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
}

enum MetricHistory<'a> {
    Model(&'a MetricsHistory),
    System(&'a AllMetricsHistory, &'static str),
}

struct MetricConfig<'a> {
    name: &'a str,
    primary_data: &'a CircularQueue<TimestampedValue>,
    secondary_data: Option<&'a CircularQueue<TimestampedValue>>,
    chart_type: charts::MetricType,
    format_fn: fn(f64) -> String,
    display_type: MetricDisplayType,
    history: MetricHistory<'a>,
}

impl MetricHistory<'_> {
    fn get_stats(
        &self,
        primary_data: &CircularQueue<TimestampedValue>,
    ) -> crate::models::MetricStats {
        match self {
            MetricHistory::Model(history) => history.get_stats(primary_data),
            MetricHistory::System(history, metric_name) => get_system_stats(metric_name, history),
        }
    }

    fn build_submenu(
        &self,
        insights: &crate::models::MetricStats,
        primary_data: &CircularQueue<TimestampedValue>,
        secondary_data: Option<&CircularQueue<TimestampedValue>>,
        format_fn: fn(f64) -> String,
        display_type: &MetricDisplayType,
    ) -> Vec<MenuItem> {
        match self {
            MetricHistory::Model(history) => build_submenu(
                insights,
                primary_data,
                secondary_data,
                format_fn,
                display_type,
                Some(history),
                None,
            ),
            MetricHistory::System(history, _) => build_submenu(
                insights,
                primary_data,
                secondary_data,
                format_fn,
                display_type,
                None,
                Some(history),
            ),
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

    fn add_title(&mut self, display_state: DisplayState) {
        let icon = icons::get_display_state_icon(display_state);
        let item = ContentItem::new("").image(icon.clone()).unwrap();
        self.items.push(MenuItem::Content(item));
    }

    fn add_status_message(&mut self, display_state: DisplayState) {
        let message = display_state.status_message();
        let color = get_hex_color(display_state.icon_color());
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

    fn add_model_metrics_section(
        &mut self,
        model_name: &str,
        history: &MetricsHistory,
        current_metrics: &crate::models::Metrics,
    ) {
        self.add_header(model_name);

        if let Some(item) = Self::create_metric(&MetricConfig {
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

        if let Some(item) = Self::create_metric(&MetricConfig {
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

        self.add_queue_status(current_metrics, history);
    }

    fn add_system_metrics_section(&mut self, history: &AllMetricsHistory) {
        let has_cpu = !history.cpu_usage_percent.is_empty();
        let has_memory =
            !history.memory_usage_percent.is_empty() && !history.used_memory_gb.is_empty();
        let has_llama_memory =
            !history.total_llama_memory_mb.is_empty() && !history.used_memory_gb.is_empty();

        // Only show header if we have any metrics to display
        if !has_cpu && !has_memory && !has_llama_memory {
            return;
        }

        self.add_header("System Metrics");

        if has_cpu {
            if let Some(item) = Self::create_metric(&MetricConfig {
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

        if has_memory {
            if let Some(item) = Self::create_metric(&MetricConfig {
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

        if has_llama_memory {
            self.add_llama_process_breakdown(history);
        }
    }

    fn add_llama_process_breakdown(&mut self, history: &AllMetricsHistory) {
        let system = sysinfo::System::new_all();
        let processes = crate::metrics::get_detailed_llama_processes(&system);

        if processes.is_empty() {
            return;
        }

        let total_memory_mb: f64 = processes.iter().map(|p| p.memory_mb).sum();

        // Create main header item with chart
        let header_text = format!("Llama Processes: {}", format_memory_mb(total_memory_mb));
        let mut header_item = ContentItem::new(header_text);

        // Add the memory trend chart
        if !history.total_llama_memory_mb.is_empty() {
            add_chart(
                &mut header_item,
                &history.total_llama_memory_mb,
                charts::MetricType::Memory,
            );
        }

        // Create submenu with process details
        let mut submenu = Vec::new();

        for process in &processes {
            let process_text = if let Some(ref model) = process.inferred_model {
                format!(
                    "├─ {} ({}): {} - {}",
                    process.name,
                    process.pid,
                    format_memory_mb(process.memory_mb),
                    model
                )
            } else {
                format!(
                    "├─ {} ({}): {}",
                    process.name,
                    process.pid,
                    format_memory_mb(process.memory_mb)
                )
            };

            submenu.push(MenuItem::Content(ContentItem::new(process_text)));
        }

        // Add total summary at the end
        submenu.push(MenuItem::Sep);
        submenu.push(MenuItem::Content(ContentItem::new(format!(
            "Total: {} across {} process{}",
            format_memory_mb(total_memory_mb),
            processes.len(),
            if processes.len() == 1 { "" } else { "es" }
        ))));

        header_item = header_item.sub(submenu);
        self.items.push(MenuItem::Content(header_item));
    }

    fn create_metric(config: &MetricConfig) -> Option<MenuItem> {
        if config.primary_data.is_empty() {
            return None;
        }

        let insights = config.history.get_stats(config.primary_data);
        let label = build_label(
            config.name,
            &insights,
            config.secondary_data,
            config.format_fn,
            &config.display_type,
        );
        let mut item = ContentItem::new(label);

        add_chart(&mut item, config.primary_data, config.chart_type);
        let submenu = config.history.build_submenu(
            &insights,
            config.primary_data,
            config.secondary_data,
            config.format_fn,
            &config.display_type,
        );
        item = item.sub(submenu);

        Some(MenuItem::Content(item))
    }

    fn add_queue_status(
        &mut self,
        current_metrics: &crate::models::Metrics,
        history: &MetricsHistory,
    ) {
        let queue_status = current_metrics.queue_status();
        let total_queue = current_metrics.requests_processing + current_metrics.requests_deferred;
        let color =
            if current_metrics.requests_processing > 0 || current_metrics.requests_deferred > 0 {
                "#FFA500"
            } else {
                "#666666"
            };

        let mut queue_item = create_colored_item(&format!("Queue: {queue_status}"), color);

        // Add the queue size chart if we have history data
        if !history.queue_size.is_empty() {
            add_chart(
                &mut queue_item,
                &history.queue_size,
                charts::MetricType::Queue,
            );
        }

        queue_item = queue_item.sub(vec![
            MenuItem::Content(ContentItem::new(format!("Status: {queue_status}"))),
            MenuItem::Content(ContentItem::new(format!(
                "Processing: {} requests",
                current_metrics.requests_processing
            ))),
            MenuItem::Content(ContentItem::new(format!(
                "Deferred: {} requests",
                current_metrics.requests_deferred
            ))),
            MenuItem::Content(ContentItem::new(format!("Total Queue Size: {total_queue}"))),
            MenuItem::Content(ContentItem::new(format!(
                "Decode Calls: {}",
                current_metrics.n_decode_total
            ))),
        ]);

        self.items.push(MenuItem::Content(queue_item));
    }

    fn add_quick_actions_section(
        &mut self,
        display_state: DisplayState,
        has_models: bool,
        service_status: &crate::types::ServiceStatus,
        exe_str: &str,
    ) {
        let mut actions = Vec::new();

        match display_state {
            DisplayState::ModelReady | DisplayState::ModelProcessingQueue => {
                // When models are running, prioritize Open UI for quick access
                if UI_COMMAND.is_available_for_state(display_state) {
                    if let Ok(item) = UI_COMMAND.create_item(exe_str) {
                        actions.push(item);
                    }
                }
                // Secondary action: unload models if present
                else if has_models {
                    if let Some(unload_cmd) =
                        CONTROL_COMMANDS.iter().find(|c| c.action == "do_unload")
                    {
                        if let Ok(item) = unload_cmd.create_item(exe_str) {
                            actions.push(item);
                        }
                    }
                }
            }
            DisplayState::ServiceLoadedNoModel => {
                // When service is loaded but no models, prioritize Open UI for quick access
                if UI_COMMAND.is_available_for_state(display_state) {
                    if let Ok(item) = UI_COMMAND.create_item(exe_str) {
                        actions.push(item);
                    }
                }
                // Secondary action: stop service if UI not available
                else if let Some(stop_cmd) = CONTROL_COMMANDS.iter().find(|c| c.action == "do_stop") {
                    if let Ok(item) = stop_cmd.create_item(exe_str) {
                        actions.push(item);
                    }
                }
            }
            DisplayState::ServiceStopped => {
                // When service is stopped, offer to start it
                if let Some(start_cmd) = CONTROL_COMMANDS.iter().find(|c| c.action == "do_start") {
                    if let Ok(item) = start_cmd.create_item(exe_str) {
                        actions.push(item);
                    }
                }
            }
            DisplayState::AgentNotLoaded => {
                // When agent not loaded, prioritize installation or starting
                if !service_status.plist_installed {
                    if let Ok(item) = INSTALL_COMMAND.create_item(exe_str) {
                        actions.push(item);
                    }
                } else if service_status.plist_installed && !service_status.is_fully_running() {
                    if let Some(start_cmd) =
                        CONTROL_COMMANDS.iter().find(|c| c.action == "do_start")
                    {
                        if let Ok(item) = start_cmd.create_item(exe_str) {
                            actions.push(item);
                        }
                    }
                }
            }
            DisplayState::AgentStarting => {
                // When starting, allow stopping to cancel
                if let Some(stop_cmd) = CONTROL_COMMANDS.iter().find(|c| c.action == "do_stop") {
                    if let Ok(item) = stop_cmd.create_item(exe_str) {
                        actions.push(item);
                    }
                }
            }
            DisplayState::ModelLoading => {
                // During model loading, no immediate action needed
                // Could add stop service if needed, but loading is usually quick
            }
        }

        // Only add the section if we have actions to show
        if !actions.is_empty() {
            for action in actions {
                self.items.push(MenuItem::Content(action));
            }
        }
    }

    fn add_settings_section(
        &mut self,
        display_state: DisplayState,
        has_models: bool,
        state: &PluginState,
        exe_str: &str,
    ) {
        let mut submenu = Vec::new();

        // Use the comprehensive service status from state
        let service_status = &state.service_status;
        let binary_available = crate::commands::find_llama_swap_binary().is_ok();

        // Show appropriate actions based on what's missing
        if matches!(display_state, DisplayState::AgentNotLoaded) {
            // Show system status for all AgentNotLoaded cases
            submenu.push(MenuItem::Content(
                ContentItem::new(format!(
                    "{} Binary: {}",
                    if binary_available {
                        ":checkmark.circle:"
                    } else {
                        ":xmark.circle:"
                    },
                    if binary_available {
                        "Found"
                    } else {
                        "Cannot find llama-swap in $PATH"
                    }
                ))
                .color(if binary_available {
                    "#34C759"
                } else {
                    "#FF9500"
                })
                .unwrap(),
            ));

            submenu.push(MenuItem::Content(
                ContentItem::new(format!(
                    "{} Plist: {}",
                    if service_status.plist_installed {
                        ":checkmark.circle:"
                    } else {
                        ":xmark.circle:"
                    },
                    if service_status.plist_installed {
                        "Installed"
                    } else {
                        "Click install below"
                    }
                ))
                .color(if service_status.plist_installed {
                    "#34C759"
                } else {
                    "#FF9500"
                })
                .unwrap(),
            ));

            submenu.push(MenuItem::Content(
                ContentItem::new(format!(
                    "{} Service: {}",
                    if service_status.is_fully_running() {
                        ":checkmark.circle:"
                    } else {
                        ":xmark.circle:"
                    },
                    service_status.status_description()
                ))
                .color(if service_status.is_fully_running() {
                    "#34C759"
                } else {
                    "#FF9500"
                })
                .unwrap(),
            ));

            // Show plist management actions based on actual plist state
            submenu.push(MenuItem::Sep);
            if !service_status.plist_installed {
                if let Ok(item) = INSTALL_COMMAND.create_item(exe_str) {
                    submenu.push(MenuItem::Content(item));
                }
            } else if let Ok(item) = UNINSTALL_COMMAND.create_item(exe_str) {
                submenu.push(MenuItem::Content(item));
            }
            submenu.push(MenuItem::Sep);
        } else {
            // Add control commands based on current state (only when service is installed)
            for command in CONTROL_COMMANDS {
                if command.is_available_for_state(display_state) {
                    // Special handling for unload command - only show if models are present
                    if command.action == "do_unload" && !has_models {
                        continue;
                    }

                    if let Ok(item) = command.create_item(exe_str) {
                        submenu.push(MenuItem::Content(item));
                    }
                }
            }

            // Add restart command when service is installed
            if let Ok(item) = RESTART_COMMAND.create_item(exe_str) {
                submenu.push(MenuItem::Content(item));
            }

            // Add uninstall option when service is installed
            if service_status.plist_installed {
                if let Ok(item) = UNINSTALL_COMMAND.create_item(exe_str) {
                    submenu.push(MenuItem::Content(item));
                }
            }
        }

        submenu.push(MenuItem::Sep);

        // Add UI command when API is available
        if UI_COMMAND.is_available_for_state(display_state) {
            if let Ok(item) = UI_COMMAND.create_item(exe_str) {
                submenu.push(MenuItem::Content(item));
            }
        }

        // Add file action commands
        for command in FILE_COMMANDS {
            if let Ok(item) = command.create_item(exe_str) {
                submenu.push(MenuItem::Content(item));
            }
        }

        submenu.push(MenuItem::Sep);
        submenu.push(MenuItem::Content(create_colored_item(
            "Llama-Swap Swiftbar Plugin",
            "#666666",
        )));

        // Debug actions - always available
        let refresh_item = ContentItem::new(":arrow.clockwise: Force Plugin Refresh").refresh();
        submenu.push(MenuItem::Content(refresh_item));

        // Simplified debug info
        submenu.push(MenuItem::Sep);

        // System status and state in one line (reuse calculated values)

        submenu.push(MenuItem::Content(ContentItem::new(format!(
            "Status: {:?} | Plist: {} | Binary: {} | Service: {}",
            display_state,
            if service_status.plist_installed {
                "✓"
            } else {
                "✗"
            },
            if binary_available { "✓" } else { "✗" },
            if service_status.is_fully_running() {
                "✓"
            } else {
                "✗"
            }
        ))));

        // Detailed service status for debugging
        submenu.push(MenuItem::Content(ContentItem::new(format!(
            "Service Details: Loaded: {} | Running: {} | API: {}",
            if service_status.launchctl_loaded {
                "✓"
            } else {
                "✗"
            },
            if service_status.process_running {
                "✓"
            } else {
                "✗"
            },
            if service_status.api_responsive {
                "✓"
            } else {
                "✗"
            }
        ))));

        submenu.push(MenuItem::Content(ContentItem::new(format!(
            "Polling Mode: {} | API Errors: {} | Metrics: {}",
            state.polling_mode.description(),
            state.error_count,
            if state.current_all_metrics.is_some() {
                "Yes"
            } else {
                "No"
            }
        ))));

        // Show model states if any
        if !state.model_states.is_empty() {
            for (model_name, model_state) in &state.model_states {
                submenu.push(MenuItem::Content(ContentItem::new(format!(
                    "{model_name}: {model_state:?}"
                ))));
            }
        }

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
    secondary_data: Option<&CircularQueue<TimestampedValue>>,
    format_fn: fn(f64) -> String,
    display_type: &MetricDisplayType,
) -> String {
    match display_type {
        MetricDisplayType::Simple => format!("{}: {}", name, format_fn(insights.current)),
        MetricDisplayType::SystemMemory => {
            let gb_current = secondary_data.unwrap().iter().next().unwrap().value; // Most recent value
            format!("{}: {:.1} GB ({:.1}%)", name, gb_current, insights.current)
        }
    }
}

fn add_chart(
    item: &mut ContentItem,
    data: &CircularQueue<TimestampedValue>,
    chart_type: charts::MetricType,
) {
    // Generate chart data in chronological order
    let values: Vec<f64> = data.iter().rev().map(|tv| tv.value).collect();
    if let Ok(chart) = charts::generate_sparkline(&values, chart_type) {
        if let Ok(chart_image) = icons::chart_to_menu_image(&chart) {
            // Replace item content with chart visualization
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
    primary_data: &CircularQueue<TimestampedValue>,
    secondary_data: Option<&CircularQueue<TimestampedValue>>,
    format_fn: fn(f64) -> String,
    display_type: &MetricDisplayType,
    _model_history: Option<&MetricsHistory>,
    system_history: Option<&AllMetricsHistory>,
) -> Vec<MenuItem> {
    let mut submenu = Vec::new();

    // Current value
    let current_text = match display_type {
        MetricDisplayType::SystemMemory => {
            let gb_current = secondary_data.unwrap().iter().next().unwrap().value; // Current memory usage
            format!("Current: {:.1} GB ({:.1}%)", gb_current, insights.current)
        }
        MetricDisplayType::Simple => format!("Current: {}", format_fn(insights.current)),
    };
    submenu.push(MenuItem::Content(ContentItem::new(current_text)));

    // Range and statistics
    if insights.count > 1 {
        let range_text = match display_type {
            MetricDisplayType::SystemMemory => {
                format!("Range: {:.1}% - {:.1}%", insights.min, insights.max)
            }
            MetricDisplayType::Simple => {
                format!("Range: {:.1}% - {:.1}%", insights.min, insights.max)
            }
        };
        submenu.push(MenuItem::Content(ContentItem::new(range_text)));

        match display_type {
            MetricDisplayType::SystemMemory => {
                // Add total system memory and available memory context
                let total_system_memory_gb = calculate_total_system_memory(system_history.unwrap());
                let current_used_gb = secondary_data.unwrap().iter().next().unwrap().value; // Current memory usage
                let available_gb = total_system_memory_gb - current_used_gb;

                submenu.push(MenuItem::Content(ContentItem::new(format!(
                    "Total System: {total_system_memory_gb:.1} GB"
                ))));
                submenu.push(MenuItem::Content(ContentItem::new(format!(
                    "Available: {available_gb:.1} GB"
                ))));

                // Add average with GB values
                if let Some(secondary) = secondary_data {
                    if secondary.len() > 1 {
                        let gb_values: Vec<f64> =
                            secondary.iter().rev().map(|tv| tv.value).collect(); // Chronological order for statistics
                        let gb_sum: f64 = gb_values.iter().sum();
                        let gb_avg = gb_sum / gb_values.len() as f64;
                        let avg_percent = (gb_avg / total_system_memory_gb) * 100.0;

                        submenu.push(MenuItem::Content(ContentItem::new(format!(
                            "Average: {avg_percent:.1}% ({gb_avg:.1} GB)"
                        ))));

                        let gb_max = gb_values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                        let max_percent = (gb_max / total_system_memory_gb) * 100.0;

                        submenu.push(MenuItem::Content(ContentItem::new(format!(
                            "Peak: {max_percent:.1}% ({gb_max:.1} GB)"
                        ))));
                    }
                }
            }
            MetricDisplayType::Simple => {
                let stats =
                    crate::models::DataAnalyzer::get_stats_from_circular_queue(primary_data);
                submenu.push(MenuItem::Content(ContentItem::new(format!(
                    "Average: {}",
                    format_fn(stats.mean)
                ))));
            }
        }
    }

    // Dataset duration
    let time_text = if primary_data.len() >= 2 {
        let oldest = primary_data.iter().last().unwrap().timestamp; // Earliest timestamp
        let newest = primary_data.iter().next().unwrap().timestamp; // Latest timestamp
        insights.time_context(oldest, newest)
    } else if primary_data.len() == 1 {
        insights.time_context(0, 0)
    } else {
        String::new()
    };

    if !time_text.is_empty() {
        submenu.push(MenuItem::Content(ContentItem::new(format!(
            "Dataset: {time_text}"
        ))));
    }

    submenu
}

fn calculate_total_system_memory(history: &AllMetricsHistory) -> f64 {
    if let (Some(latest_used_gb), Some(latest_percent)) = (
        history.used_memory_gb.iter().next(),
        history.memory_usage_percent.iter().next(),
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

fn format_tps(v: f64) -> String {
    format!("{v:.1} tok/s")
}

fn format_percent(v: f64) -> String {
    format!("{v:.1}%")
}

fn format_memory_mb(mb: f64) -> String {
    if mb >= 1024.0 {
        format!("{:.1} GB", mb / 1024.0)
    } else {
        format!("{mb:.0} MB")
    }
}

pub fn build_menu(state: &PluginState) -> crate::Result<String> {
    let mut menu = MenuBuilder::new();

    let display_state = state.get_display_state();

    menu.add_title(display_state);
    menu.add_separator();
    menu.add_status_message(display_state);
    menu.add_separator();

    let has_models = state
        .current_all_metrics
        .as_ref()
        .is_some_and(|m| !m.models.is_empty());

    // Show system metrics for all states where they're being collected
    menu.add_system_metrics_section(&state.metrics_history);

    if let Some(ref all_metrics) = state.current_all_metrics {
        let mut sorted_models = all_metrics.models.clone();
        sorted_models.sort_by(|a, b| a.model_name.cmp(&b.model_name));

        for model_metrics in &sorted_models {
            if let Some(model_history) = state
                .metrics_history
                .get_model_history(&model_metrics.model_name)
            {
                if !model_history.tps.is_empty() {
                    menu.add_separator();
                    menu.add_model_metrics_section(
                        &model_metrics.model_name,
                        model_history,
                        &model_metrics.metrics,
                    );
                }
            }
        }
    }

    let exe = std::env::current_exe().unwrap();
    let exe_str = exe.to_str().unwrap();

    menu.add_separator();
    menu.add_quick_actions_section(display_state, has_models, &state.service_status, exe_str);
    menu.add_settings_section(display_state, has_models, state, exe_str);

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
    use crate::state_model::AgentState;
    #[test]
    fn test_menu_with_running_service() {
        let state = create_test_state_for_running_service();

        // Verify the state is as expected
        assert!(matches!(state.agent_state, AgentState::Running));
        assert_eq!(state.get_display_state(), DisplayState::ModelReady);

        let menu_str = build_menu(&state).unwrap();

        // When service is running and models are ready, should show stop options
        assert!(menu_str.contains("Stop Llama-Swap Service"));
        assert!(!menu_str.contains("Start Llama-Swap Service"));
    }

    #[test]
    fn test_menu_with_stopped_service() {
        let state = create_test_state_for_stopped_service();

        // Verify the state is as expected
        assert!(matches!(state.agent_state, AgentState::Stopped));
        assert_eq!(state.get_display_state(), DisplayState::ServiceStopped);

        let menu_str = build_menu(&state).unwrap();

        // When service is stopped (but installed), should show start option
        assert!(menu_str.contains("Start Llama-Swap Service"));
        assert!(!menu_str.contains("Install Llama-Swap Service"));
    }

    #[test]
    fn test_menu_with_not_installed_service() {
        let state = create_test_state_for_not_installed_service();

        // Verify the state is as expected
        assert!(matches!(state.agent_state, AgentState::NotReady { .. }));
        assert_eq!(state.get_display_state(), DisplayState::AgentNotLoaded);

        let menu_str = build_menu(&state).unwrap();

        // The test shows AgentNotLoaded state even when plist exists (can happen when binary missing)
        assert!(menu_str.contains("Missing requirements"));
        // Since this tests against real system state, we can't reliably test install/uninstall commands
        // The important test is that the display state is correct
        assert!(menu_str.contains("AgentNotLoaded"));
    }

    #[test]
    fn test_error_menu() {
        let error_menu = build_error_menu("Test error message").unwrap();

        assert!(error_menu.contains("Plugin Error"));
        assert!(error_menu.contains("Test error message"));
        assert!(error_menu.contains("Retry"));
    }

    fn create_test_state_for_running_service() -> PluginState {
        use crate::models::{AllMetrics, Metrics, ModelMetrics, ModelState};
        use crate::state_model::{AgentState, ModelState as NewModelState};

        let mut state = PluginState::new().unwrap();

        // Set agent state to Running
        state.agent_state = AgentState::Running;

        // Add a running model
        state
            .model_states
            .insert("test-model".to_string(), NewModelState::Running);

        // Set up some dummy metrics to make the state consistent
        let dummy_metrics = AllMetrics {
            models: vec![ModelMetrics {
                model_name: "test-model".to_string(),
                model_state: ModelState::Running,
                metrics: Metrics {
                    prompt_tokens_per_sec: 10.0,
                    predicted_tokens_per_sec: 15.0,
                    requests_processing: 0,
                    requests_deferred: 0,

                    n_decode_total: 100,
                    memory_mb: 1000.0,
                },
            }],
        };
        state.current_all_metrics = Some(dummy_metrics);

        state
    }

    fn create_test_state_for_stopped_service() -> PluginState {
        use crate::state_model::AgentState;

        let mut state = PluginState::new().unwrap();

        // Set agent state to Stopped (service installed but not running)
        state.agent_state = AgentState::Stopped;

        // No models since service is not running
        state.model_states.clear();

        // No metrics since service is stopped
        state.current_all_metrics = None;

        state
    }

    fn create_test_state_for_not_installed_service() -> PluginState {
        use crate::state_model::{AgentState, NotReadyReason};

        let mut state = PluginState::new().unwrap();

        // Set agent state to NotReady (simulating service not installed)
        state.agent_state = AgentState::NotReady {
            reason: NotReadyReason::PlistMissing,
        };

        // No models since service is not running
        state.model_states.clear();

        // No metrics since service is stopped
        state.current_all_metrics = None;

        state
    }
}
