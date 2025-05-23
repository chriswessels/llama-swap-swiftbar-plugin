use bitbar::{Menu, MenuItem};
use crate::PluginState;
use crate::models::ServiceStatus;

pub fn build_menu(state: &PluginState) -> crate::Result<String> {
    let mut items = vec![];
    
    // Title with status
    let title = match state.current_status {
        ServiceStatus::Running => "🟢 Running",
        ServiceStatus::Stopped => "🔴 Stopped",
        ServiceStatus::Unknown => "⚪ Unknown",
    };
    
    items.push(MenuItem::new(title));
    items.push(MenuItem::Sep);
    items.push(MenuItem::new("Llama-Swap SwiftBar Plugin"));
    items.push(MenuItem::new(format!("Status: {:?}", state.current_status)));
    
    let menu = Menu(items);
    Ok(menu.to_string())
}

pub fn build_error_menu(message: &str) -> Result<String, std::fmt::Error> {
    let menu = Menu(vec![
        MenuItem::new("⚠️ Error"),
        MenuItem::Sep,
        MenuItem::new(message),
    ]);
    Ok(menu.to_string())
}