use smlang::statemachine;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelLoading(pub bool);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelActive(pub bool);

statemachine! {
    name: Model,
    derive_states: [Debug, Clone, Copy],
    derive_events: [Debug, Clone, Copy],
    transitions: {
        *Unknown + LoadingUpdate(ModelLoading) [model_is_loading] = Loading,
        Unknown + LoadingUpdate(ModelLoading) [!model_is_loading] = Unknown,
        Unknown + ActiveUpdate(ModelActive) [model_is_active] = Running,
        Unknown + ActiveUpdate(ModelActive) [!model_is_active] = Unknown,
        
        Loading + LoadingUpdate(ModelLoading) [model_is_loading] = Loading,
        Loading + LoadingUpdate(ModelLoading) [!model_is_loading] = Unknown,
        Loading + ActiveUpdate(ModelActive) [model_is_active] = Running,
        Loading + ActiveUpdate(ModelActive) [!model_is_active] = Unknown,
        
        Running + LoadingUpdate(ModelLoading) [model_is_loading] = Loading,
        Running + LoadingUpdate(ModelLoading) [!model_is_loading] = Running,
        Running + ActiveUpdate(ModelActive) [model_is_active] = Running,
        Running + ActiveUpdate(ModelActive) [!model_is_active] = Unknown,
    }
}

pub struct ModelContext;

impl Default for ModelContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelContext {
    pub fn new() -> Self {
        Self
    }
}

impl ModelStateMachineContext for ModelContext {
    fn model_is_loading(&self, event: &ModelLoading) -> Result<bool, ()> {
        Ok(event.0)
    }

    fn model_is_active(&self, event: &ModelActive) -> Result<bool, ()> {
        Ok(event.0)
    }
}

impl ModelStates {
    pub fn is_loading(&self) -> bool {
        matches!(self, ModelStates::Loading)
    }
    
    pub fn is_running(&self) -> bool {
        matches!(self, ModelStates::Running)
    }
    
    pub fn is_unknown(&self) -> bool {
        matches!(self, ModelStates::Unknown)
    }
}