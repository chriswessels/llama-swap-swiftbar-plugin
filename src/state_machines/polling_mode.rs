use smlang::statemachine;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StateChange;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QueueActivity(pub bool);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MinimumDurationElapsed;

statemachine! {
    name: PollingMode,
    derive_states: [Debug, Clone, Copy],
    derive_events: [Debug, Clone, Copy],
    transitions: {
        *Idle + StateChangeDetected(StateChange) / start_transition_timer = Starting,
        Idle + ActivityCheck(QueueActivity) [has_activity] = Active,
        Idle + ActivityCheck(QueueActivity) [!has_activity] = Idle,
        
        Active + StateChangeDetected(StateChange) / start_transition_timer = Starting,
        Active + ActivityCheck(QueueActivity) [has_activity] = Active,
        Active + ActivityCheck(QueueActivity) [!has_activity] = Idle,
        
        Starting + StateChangeDetected(StateChange) / restart_transition_timer = Starting,
        Starting + MinimumDurationComplete(MinimumDurationElapsed) [duration_elapsed_with_activity] = Active,
        Starting + MinimumDurationComplete(MinimumDurationElapsed) [duration_elapsed_without_activity] = Idle,
        Starting + ActivityCheck(QueueActivity) [minimum_duration_elapsed && has_activity] = Active,
        Starting + ActivityCheck(QueueActivity) [minimum_duration_elapsed && !has_activity] = Idle,
        Starting + ActivityCheck(QueueActivity) [!minimum_duration_elapsed] = Starting,
    }
}

pub struct PollingModeContext {
    pub transition_start_time: Option<Instant>,
    pub minimum_duration: Duration,
    pub current_activity: bool,
}

impl PollingModeContext {
    pub fn new() -> Self {
        Self {
            transition_start_time: None,
            minimum_duration: Duration::from_secs(crate::constants::MIN_STARTING_DURATION_SECS),
            current_activity: false,
        }
    }

    pub fn update_activity(&mut self, has_activity: bool) {
        self.current_activity = has_activity;
    }

    pub fn should_complete_minimum_duration(&self) -> bool {
        match self.transition_start_time {
            Some(start_time) => start_time.elapsed() >= self.minimum_duration,
            None => true, // If no timer set, consider duration elapsed
        }
    }
}

impl PollingModeStateMachineContext for PollingModeContext {
    fn has_activity(&self, event: &QueueActivity) -> Result<bool, ()> {
        Ok(event.0)
    }

    fn duration_elapsed_with_activity(&self, _event: &MinimumDurationElapsed) -> Result<bool, ()> {
        Ok(self.should_complete_minimum_duration() && self.current_activity)
    }

    fn duration_elapsed_without_activity(&self, _event: &MinimumDurationElapsed) -> Result<bool, ()> {
        Ok(self.should_complete_minimum_duration() && !self.current_activity)
    }

    fn minimum_duration_elapsed(&self, _event: &QueueActivity) -> Result<bool, ()> {
        Ok(self.should_complete_minimum_duration())
    }

    fn start_transition_timer(&mut self, _event: StateChange) -> Result<(), ()> {
        self.transition_start_time = Some(Instant::now());
        Ok(())
    }

    fn restart_transition_timer(&mut self, _event: StateChange) -> Result<(), ()> {
        self.transition_start_time = Some(Instant::now());
        Ok(())
    }
}

impl PollingModeStates {
    pub fn interval_secs(&self) -> u64 {
        match self {
            PollingModeStates::Idle => crate::constants::UPDATE_INTERVAL_SECS,
            PollingModeStates::Active => crate::constants::ACTIVE_INTERVAL_SECS,
            PollingModeStates::Starting => crate::constants::STARTING_INTERVAL_SECS,
        }
    }
    
    pub fn description(&self) -> &'static str {
        match self {
            PollingModeStates::Idle => "idle polling",
            PollingModeStates::Active => "active polling",
            PollingModeStates::Starting => "transition polling",
        }
    }
}