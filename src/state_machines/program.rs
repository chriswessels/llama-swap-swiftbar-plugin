use smlang::statemachine;
use crate::state_machines::agent::AgentStates;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentUpdate(pub AgentStates);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelUpdate {
    pub has_models: bool,
    pub has_loading: bool,
    pub has_activity: bool,
}

statemachine! {
    name: Program,
    derive_states: [Debug, Clone, Copy],
    derive_events: [Debug, Clone, Copy],
    transitions: {
        *AgentNotLoaded + AgentStateChanged(AgentUpdate) [agent_is_stopped_or_not_installed] = AgentNotLoaded,
        AgentNotLoaded + AgentStateChanged(AgentUpdate) [agent_is_starting] = AgentStarting,
        AgentNotLoaded + AgentStateChanged(AgentUpdate) [agent_is_running] / check_models = ServiceLoadedNoModel,
        
        AgentStarting + AgentStateChanged(AgentUpdate) [agent_is_stopped_or_not_installed] = AgentNotLoaded,
        AgentStarting + AgentStateChanged(AgentUpdate) [agent_is_starting] = AgentStarting,
        AgentStarting + AgentStateChanged(AgentUpdate) [agent_is_running] / check_models = ServiceLoadedNoModel,
        
        ServiceLoadedNoModel + AgentStateChanged(AgentUpdate) [agent_is_stopped_or_not_installed] = AgentNotLoaded,
        ServiceLoadedNoModel + AgentStateChanged(AgentUpdate) [agent_is_starting] = AgentStarting,
        ServiceLoadedNoModel + ModelStateChanged(ModelUpdate) [has_models && has_loading_models] = ModelLoading,
        ServiceLoadedNoModel + ModelStateChanged(ModelUpdate) [has_models && !has_loading_models && has_queue_activity] = ModelProcessingQueue,
        ServiceLoadedNoModel + ModelStateChanged(ModelUpdate) [has_models && !has_loading_models && !has_queue_activity] = ModelReady,
        ServiceLoadedNoModel + ModelStateChanged(ModelUpdate) [!has_models] = ServiceLoadedNoModel,
        
        ModelLoading + AgentStateChanged(AgentUpdate) [agent_is_stopped_or_not_installed] = AgentNotLoaded,
        ModelLoading + AgentStateChanged(AgentUpdate) [agent_is_starting] = AgentStarting,
        ModelLoading + ModelStateChanged(ModelUpdate) [!has_models] = ServiceLoadedNoModel,
        ModelLoading + ModelStateChanged(ModelUpdate) [has_models && has_loading_models] = ModelLoading,
        ModelLoading + ModelStateChanged(ModelUpdate) [has_models && !has_loading_models && has_queue_activity] = ModelProcessingQueue,
        ModelLoading + ModelStateChanged(ModelUpdate) [has_models && !has_loading_models && !has_queue_activity] = ModelReady,
        
        ModelProcessingQueue + AgentStateChanged(AgentUpdate) [agent_is_stopped_or_not_installed] = AgentNotLoaded,
        ModelProcessingQueue + AgentStateChanged(AgentUpdate) [agent_is_starting] = AgentStarting,
        ModelProcessingQueue + ModelStateChanged(ModelUpdate) [!has_models] = ServiceLoadedNoModel,
        ModelProcessingQueue + ModelStateChanged(ModelUpdate) [has_models && has_loading_models] = ModelLoading,
        ModelProcessingQueue + ModelStateChanged(ModelUpdate) [has_models && !has_loading_models && has_queue_activity] = ModelProcessingQueue,
        ModelProcessingQueue + ModelStateChanged(ModelUpdate) [has_models && !has_loading_models && !has_queue_activity] = ModelReady,
        
        ModelReady + AgentStateChanged(AgentUpdate) [agent_is_stopped_or_not_installed] = AgentNotLoaded,
        ModelReady + AgentStateChanged(AgentUpdate) [agent_is_starting] = AgentStarting,
        ModelReady + ModelStateChanged(ModelUpdate) [!has_models] = ServiceLoadedNoModel,
        ModelReady + ModelStateChanged(ModelUpdate) [has_models && has_loading_models] = ModelLoading,
        ModelReady + ModelStateChanged(ModelUpdate) [has_models && !has_loading_models && has_queue_activity] = ModelProcessingQueue,
        ModelReady + ModelStateChanged(ModelUpdate) [has_models && !has_loading_models && !has_queue_activity] = ModelReady,
    }
}

pub struct ProgramContext;

impl ProgramContext {
    pub fn new() -> Self {
        Self
    }
}

impl ProgramStateMachineContext for ProgramContext {
    fn agent_is_stopped_or_not_installed(&self, event: &AgentUpdate) -> Result<bool, ()> {
        Ok(matches!(event.0, AgentStates::NotInstalled | AgentStates::Stopped))
    }

    fn agent_is_starting(&self, event: &AgentUpdate) -> Result<bool, ()> {
        Ok(matches!(event.0, AgentStates::Starting))
    }

    fn agent_is_running(&self, event: &AgentUpdate) -> Result<bool, ()> {
        Ok(matches!(event.0, AgentStates::Running))
    }

    fn has_models(&self, event: &ModelUpdate) -> Result<bool, ()> {
        Ok(event.has_models)
    }

    fn has_loading_models(&self, event: &ModelUpdate) -> Result<bool, ()> {
        Ok(event.has_loading)
    }

    fn has_queue_activity(&self, event: &ModelUpdate) -> Result<bool, ()> {
        Ok(event.has_activity)
    }

    fn check_models(&mut self, _event: AgentUpdate) -> Result<(), ()> {
        // This action could trigger model checking if needed
        Ok(())
    }
}

impl ProgramStates {
    pub fn icon_color(&self) -> &'static str {
        match self {
            ProgramStates::ModelProcessingQueue => "blue",
            ProgramStates::ModelReady => "green",
            ProgramStates::ModelLoading => "yellow",
            ProgramStates::ServiceLoadedNoModel => "grey",
            ProgramStates::AgentStarting => "yellow",
            ProgramStates::AgentNotLoaded => "red",
        }
    }
    
    pub fn status_message(&self) -> &'static str {
        match self {
            ProgramStates::ModelProcessingQueue => "Model actively processing queue",
            ProgramStates::ModelReady => "Model ready, queue empty",
            ProgramStates::ModelLoading => "Model loading",
            ProgramStates::ServiceLoadedNoModel => "Service loaded, no model loaded",
            ProgramStates::AgentStarting => "Agent starting",
            ProgramStates::AgentNotLoaded => "Agent not loaded",
        }
    }
}