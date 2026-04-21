use nova_rt::{NovaAgentId, NovaServiceId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AgentCapabilityBundle {
    pub tool_grants: u16,
    pub service_grants: u16,
    pub memory_budget_pages: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AgentDescriptor {
    pub id: NovaAgentId,
    pub name: &'static str,
    pub owner_service: NovaServiceId,
    pub capabilities: AgentCapabilityBundle,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum AgentLifecycleState {
    Declared = 0,
    Starting = 1,
    Running = 2,
    Stopping = 3,
    Stopped = 4,
    Failed = 5,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum AgentControlEvent {
    Launch = 1,
    Ready = 2,
    Stop = 3,
    Fail = 4,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AgentStateMachine {
    pub descriptor: AgentDescriptor,
    pub state: AgentLifecycleState,
}

impl AgentStateMachine {
    pub const fn new(descriptor: AgentDescriptor) -> Self {
        Self {
            descriptor,
            state: AgentLifecycleState::Declared,
        }
    }

    pub const fn apply(self, event: AgentControlEvent) -> Self {
        let state = match (self.state, event) {
            (AgentLifecycleState::Declared, AgentControlEvent::Launch) => {
                AgentLifecycleState::Starting
            }
            (AgentLifecycleState::Starting, AgentControlEvent::Ready) => {
                AgentLifecycleState::Running
            }
            (AgentLifecycleState::Running, AgentControlEvent::Stop) => {
                AgentLifecycleState::Stopping
            }
            (AgentLifecycleState::Stopping, AgentControlEvent::Ready) => {
                AgentLifecycleState::Stopped
            }
            (_, AgentControlEvent::Fail) => AgentLifecycleState::Failed,
            _ => self.state,
        };
        Self { state, ..self }
    }
}
