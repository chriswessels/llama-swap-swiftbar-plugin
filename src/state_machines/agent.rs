use smlang::statemachine;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ServiceRunning(pub bool);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StartupTimeout;

statemachine! {
    name: Agent,
    derive_states: [Debug, Clone, Copy],
    derive_events: [Debug, Clone, Copy],
    transitions: {
        *NotInstalled + ServiceDetected(ServiceRunning) [service_is_running] / start_transition_timer = Starting,
        NotInstalled + ServiceDetected(ServiceRunning) [!service_is_running] = NotInstalled,
        
        Stopped + ServiceDetected(ServiceRunning) [service_is_running] / start_transition_timer = Starting,
        Stopped + ServiceDetected(ServiceRunning) [!service_is_running] = Stopped,
        
        Starting + ServiceDetected(ServiceRunning) [service_is_running] = Starting,
        Starting + ServiceDetected(ServiceRunning) [!service_is_running] = Stopped,
        Starting + StartupComplete(StartupTimeout) [service_still_running] = Running,
        Starting + StartupComplete(StartupTimeout) [!service_still_running] = Stopped,
        
        Running + ServiceDetected(ServiceRunning) [service_is_running] = Running,
        Running + ServiceDetected(ServiceRunning) [!service_is_running] = Stopped,
    }
}

pub struct AgentContext {
    pub transition_start_time: Option<Instant>,
    pub startup_timeout: Duration,
}

impl AgentContext {
    pub fn new() -> Self {
        Self {
            transition_start_time: None,
            startup_timeout: Duration::from_secs(5),
        }
    }

    pub fn should_complete_startup(&self) -> bool {
        match self.transition_start_time {
            Some(start_time) => start_time.elapsed() >= self.startup_timeout,
            None => false,
        }
    }
}

impl AgentStateMachineContext for AgentContext {
    fn service_is_running(&self, event: &ServiceRunning) -> Result<bool, ()> {
        Ok(event.0)
    }

    fn service_still_running(&self, _event: &StartupTimeout) -> Result<bool, ()> {
        // This would typically check if the service is still running
        // For now, assume it's still running if we've started the timer
        Ok(self.transition_start_time.is_some())
    }

    fn start_transition_timer(&mut self, _event: ServiceRunning) -> Result<(), ()> {
        self.transition_start_time = Some(Instant::now());
        Ok(())
    }
}

impl AgentStates {
    pub fn icon_color(&self) -> &'static str {
        match self {
            AgentStates::NotInstalled | AgentStates::Stopped => "red",
            AgentStates::Starting => "yellow", 
            AgentStates::Running => "green",
        }
    }
    
    pub fn description(&self) -> &'static str {
        match self {
            AgentStates::NotInstalled => "Agent not installed",
            AgentStates::Stopped => "Agent stopped",
            AgentStates::Starting => "Agent starting",
            AgentStates::Running => "Agent running",
        }
    }
}

// Re-export the generated types for convenience