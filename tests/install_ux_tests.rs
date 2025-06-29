use llama_swap_swiftbar::menu::build_menu;
use llama_swap_swiftbar::state_model::{AgentState, NotReadyReason};
use llama_swap_swiftbar::types::PluginState;

#[test]
fn test_install_ux_message_contains_helpful_guidance() {
    // Create a mock state where plist is not installed
    let state = create_test_state_binary_missing();

    let menu_str = build_menu(&state).unwrap();

    // The test should verify that when the agent is not loaded,
    // the menu shows install options
    assert!(menu_str.contains("Install Llama-Swap Service"));

    // The menu should also show the missing requirements status
    assert!(menu_str.contains("Missing requirements"));

    // The advanced submenu should show binary detection status
    // (this shows regardless of whether binary is actually available)
    assert!(menu_str.contains("Binary:"));

    println!("✓ Test passed: AgentNotLoaded state shows install options");
}

#[test]
fn test_install_button_shown_when_binary_available() {
    // This test will only pass if llama-swap binary is actually available
    if llama_swap_swiftbar::commands::find_llama_swap_binary().is_err() {
        // Skip this test if binary is not available
        return;
    }

    let state = create_test_state_binary_available();

    let menu_str = build_menu(&state).unwrap();

    // When binary is available but plist not installed, should show install button
    assert!(menu_str.contains("Install Llama-Swap Service"));
}

fn create_test_state_binary_missing() -> PluginState {
    let mut state = PluginState::new().unwrap();

    // Set agent state to NotReady due to both binary and plist missing
    state.agent_state = AgentState::NotReady {
        reason: NotReadyReason::PlistMissing,
    };

    // Ensure service status reflects missing components
    state.service_status.plist_installed = false;
    state.service_status.launchctl_loaded = false;
    state.service_status.process_running = false;
    state.service_status.api_responsive = false;

    state
}

fn create_test_state_binary_available() -> PluginState {
    let mut state = PluginState::new().unwrap();

    // Set agent state to NotReady due to plist missing (but binary available)
    state.agent_state = AgentState::NotReady {
        reason: NotReadyReason::PlistMissing,
    };

    // Ensure service status reflects only plist missing
    state.service_status.plist_installed = false;
    state.service_status.launchctl_loaded = false;
    state.service_status.process_running = false;
    state.service_status.api_responsive = false;

    state
}
