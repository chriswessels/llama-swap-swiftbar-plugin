use bitbar::{Menu, MenuItem, ContentItem, attr};
use crate::{PluginState, icons, charts};
use crate::models::ServiceStatus;

pub fn build_menu(state: &PluginState) -> crate::Result<String> {
    let mut items = vec![];
    
    // Generate and add title with status icon
    if let Ok(status_icon) = generate_title_item(state.current_status) {
        items.push(status_icon);
    }
    
    items.push(MenuItem::Sep);
    
    // Add service controls
    items.extend(generate_control_items(state.current_status));
    
    items.push(MenuItem::Sep);
    
    // Add metrics if service is running
    if state.current_status == ServiceStatus::Running {
        items.extend(generate_metrics_items(&state.metrics_history));
    } else {
        let mut item = ContentItem::new("Service is not running");
        item = item.color("#888888").unwrap();
        items.push(MenuItem::Content(item));
    }
    
    let menu = Menu(items);
    Ok(menu.to_string())
}

fn generate_title_item(status: ServiceStatus) -> crate::Result<MenuItem> {
    // Generate icon with status dot
    let icon = icons::generate_status_icon(status)?;
    let menu_image = icons::icon_to_menu_image(icon)?;
    
    // Create title item with just the icon (no text)
    let item = ContentItem::new("")
        .image(menu_image).unwrap();
    Ok(MenuItem::Content(item))
}

fn generate_control_items(status: ServiceStatus) -> Vec<MenuItem> {
    let mut items = vec![];
    
    match status {
        ServiceStatus::Running => {
            let mut item = ContentItem::new("🔴 Stop Llama-Swap");
            let exe = std::env::current_exe().unwrap();
            let exe_str = exe.to_str().unwrap();
            item = item.command(attr::Command::try_from((exe_str, "do_stop")).unwrap()).unwrap();
            items.push(MenuItem::Content(item));
        }
        ServiceStatus::Stopped | ServiceStatus::Unknown => {
            let mut item = ContentItem::new("🟢 Start Llama-Swap");
            let exe = std::env::current_exe().unwrap();
            let exe_str = exe.to_str().unwrap();
            item = item.command(attr::Command::try_from((exe_str, "do_start")).unwrap()).unwrap();
            items.push(MenuItem::Content(item));
        }
    }
    
    let mut restart = ContentItem::new("⟲ Restart Llama-Swap");
    let exe = std::env::current_exe().unwrap();
    let exe_str = exe.to_str().unwrap();
    restart = restart.command(attr::Command::try_from((exe_str, "do_restart")).unwrap()).unwrap();
    items.push(MenuItem::Content(restart));
    
    items
}

fn generate_metrics_items(history: &crate::models::MetricsHistory) -> Vec<MenuItem> {
    let mut items = vec![];
    
    // Section header
    let mut header = ContentItem::new("Performance Metrics");
    header = header.color("#888888").unwrap();
    items.push(MenuItem::Content(header));
    
    // TPS with sparkline
    if let Some(&latest_tps) = history.tps.back() {
        if let Ok(sparkline) = charts::generate_tps_sparkline(&history.tps) {
            if let Ok(chart_image) = icons::icon_to_menu_image(sparkline) {
                let mut item = ContentItem::new(format!("TPS: {:.1}", latest_tps));
                item = item.image(chart_image).unwrap();
                items.push(MenuItem::Content(item));
            }
        }
    }
    
    // Memory with sparkline
    if let Some(&latest_mem) = history.memory_mb.back() {
        if let Ok(sparkline) = charts::generate_memory_sparkline(&history.memory_mb) {
            if let Ok(chart_image) = icons::icon_to_menu_image(sparkline) {
                let mut item = ContentItem::new(format!("Memory: {:.1} MB", latest_mem));
                item = item.image(chart_image).unwrap();
                items.push(MenuItem::Content(item));
            }
        }
    }
    
    // Cache hit rate with sparkline
    if let Some(&latest_cache) = history.cache_hit_rate.back() {
        if let Ok(sparkline) = charts::generate_cache_sparkline(&history.cache_hit_rate) {
            if let Ok(chart_image) = icons::icon_to_menu_image(sparkline) {
                let mut item = ContentItem::new(format!("Cache Hit Rate: {:.1}%", latest_cache));
                item = item.image(chart_image).unwrap();
                items.push(MenuItem::Content(item));
            }
        }
    }
    
    items
}

pub fn build_error_menu(message: &str) -> Result<String, std::fmt::Error> {
    let mut error_item = ContentItem::new(message);
    error_item = error_item.color("#ff0000").unwrap();
    
    let menu = Menu(vec![
        MenuItem::Content(ContentItem::new("⚠️ Error")),
        MenuItem::Sep,
        MenuItem::Content(error_item),
    ]);
    Ok(menu.to_string())
}