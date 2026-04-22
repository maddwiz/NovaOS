use nova_rt::{NovaAgentId, NovaSceneId, NovaServiceId};

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

impl AgentLifecycleState {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Declared => "declared",
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Stopping => "stopping",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum AgentControlEvent {
    Launch = 1,
    Ready = 2,
    Stop = 3,
    Fail = 4,
}

impl AgentControlEvent {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Launch => "launch",
            Self::Ready => "ready",
            Self::Stop => "stop",
            Self::Fail => "fail",
        }
    }
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

    pub const fn is_running(self) -> bool {
        matches!(self.state, AgentLifecycleState::Running)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AgentRuntimeRecord {
    pub machine: AgentStateMachine,
    pub scene: NovaSceneId,
    pub used_tool_grants: u16,
    pub delegated_services: u16,
    pub used_memory_pages: u64,
}

impl AgentRuntimeRecord {
    pub const fn new(machine: AgentStateMachine, scene: NovaSceneId) -> Self {
        Self {
            machine,
            scene,
            used_tool_grants: 0,
            delegated_services: 0,
            used_memory_pages: 0,
        }
    }

    pub const fn from_descriptor(descriptor: AgentDescriptor, scene: NovaSceneId) -> Self {
        Self::new(AgentStateMachine::new(descriptor), scene)
    }

    pub const fn apply(self, event: AgentControlEvent) -> Self {
        Self {
            machine: self.machine.apply(event),
            ..self
        }
    }

    pub const fn with_usage(
        self,
        used_tool_grants: u16,
        delegated_services: u16,
        used_memory_pages: u64,
    ) -> Self {
        Self {
            used_tool_grants,
            delegated_services,
            used_memory_pages,
            ..self
        }
    }

    pub const fn is_running(self) -> bool {
        self.machine.is_running()
    }

    pub const fn quota_snapshot(self) -> AgentQuotaSnapshot {
        let capabilities = self.machine.descriptor.capabilities;
        AgentQuotaSnapshot {
            agent: self.machine.descriptor.id,
            tool_grants: capabilities.tool_grants,
            used_tool_grants: self.used_tool_grants,
            service_grants: capabilities.service_grants,
            delegated_services: self.delegated_services,
            memory_budget_pages: capabilities.memory_budget_pages,
            used_memory_pages: self.used_memory_pages,
        }
    }

    pub const fn check_tool_grants(self, requested: u16) -> AgentQuotaDecision {
        let snapshot = self.quota_snapshot();
        let status = if !self.is_running() {
            AgentQuotaStatus::NotRunning
        } else if !u16_budget_allows(self.used_tool_grants, requested, snapshot.tool_grants) {
            AgentQuotaStatus::ToolGrantExceeded
        } else {
            AgentQuotaStatus::Allowed
        };

        AgentQuotaDecision {
            agent: snapshot.agent,
            scene: self.scene,
            status,
            requested: requested as u64,
            used: self.used_tool_grants as u64,
            limit: snapshot.tool_grants as u64,
        }
    }

    pub const fn check_service_delegation(self, requested: u16) -> AgentQuotaDecision {
        let snapshot = self.quota_snapshot();
        let status = if !self.is_running() {
            AgentQuotaStatus::NotRunning
        } else if !u16_budget_allows(self.delegated_services, requested, snapshot.service_grants) {
            AgentQuotaStatus::ServiceGrantExceeded
        } else {
            AgentQuotaStatus::Allowed
        };

        AgentQuotaDecision {
            agent: snapshot.agent,
            scene: self.scene,
            status,
            requested: requested as u64,
            used: self.delegated_services as u64,
            limit: snapshot.service_grants as u64,
        }
    }

    pub const fn check_memory_pages(self, requested_pages: u64) -> AgentQuotaDecision {
        let snapshot = self.quota_snapshot();
        let status = if !self.is_running() {
            AgentQuotaStatus::NotRunning
        } else if !u64_budget_allows(
            self.used_memory_pages,
            requested_pages,
            snapshot.memory_budget_pages,
        ) {
            AgentQuotaStatus::MemoryBudgetExceeded
        } else {
            AgentQuotaStatus::Allowed
        };

        AgentQuotaDecision {
            agent: snapshot.agent,
            scene: self.scene,
            status,
            requested: requested_pages,
            used: self.used_memory_pages,
            limit: snapshot.memory_budget_pages,
        }
    }

    pub const fn scene_participation(
        self,
        requested_scene: NovaSceneId,
    ) -> AgentSceneParticipation {
        let status = if !self.is_running() {
            AgentSceneParticipationStatus::NotRunning
        } else if self.scene.0 == requested_scene.0 {
            AgentSceneParticipationStatus::Attached
        } else {
            AgentSceneParticipationStatus::SceneMismatch
        };

        AgentSceneParticipation {
            agent: self.machine.descriptor.id,
            current_scene: self.scene,
            requested_scene,
            status,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AgentQuotaSnapshot {
    pub agent: NovaAgentId,
    pub tool_grants: u16,
    pub used_tool_grants: u16,
    pub service_grants: u16,
    pub delegated_services: u16,
    pub memory_budget_pages: u64,
    pub used_memory_pages: u64,
}

impl AgentQuotaSnapshot {
    pub const fn remaining_tool_grants(self) -> u16 {
        if self.used_tool_grants >= self.tool_grants {
            0
        } else {
            self.tool_grants - self.used_tool_grants
        }
    }

    pub const fn remaining_service_grants(self) -> u16 {
        if self.delegated_services >= self.service_grants {
            0
        } else {
            self.service_grants - self.delegated_services
        }
    }

    pub const fn remaining_memory_pages(self) -> u64 {
        if self.used_memory_pages >= self.memory_budget_pages {
            0
        } else {
            self.memory_budget_pages - self.used_memory_pages
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum AgentQuotaStatus {
    Allowed = 1,
    NotRunning = 2,
    ToolGrantExceeded = 3,
    ServiceGrantExceeded = 4,
    MemoryBudgetExceeded = 5,
}

impl AgentQuotaStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Allowed => "allowed",
            Self::NotRunning => "not-running",
            Self::ToolGrantExceeded => "tool-grant-exceeded",
            Self::ServiceGrantExceeded => "service-grant-exceeded",
            Self::MemoryBudgetExceeded => "memory-budget-exceeded",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AgentQuotaDecision {
    pub agent: NovaAgentId,
    pub scene: NovaSceneId,
    pub status: AgentQuotaStatus,
    pub requested: u64,
    pub used: u64,
    pub limit: u64,
}

impl AgentQuotaDecision {
    pub const fn allowed(self) -> bool {
        matches!(self.status, AgentQuotaStatus::Allowed)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum AgentSceneParticipationStatus {
    Attached = 1,
    SceneMismatch = 2,
    NotRunning = 3,
}

impl AgentSceneParticipationStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Attached => "attached",
            Self::SceneMismatch => "scene-mismatch",
            Self::NotRunning => "not-running",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AgentSceneParticipation {
    pub agent: NovaAgentId,
    pub current_scene: NovaSceneId,
    pub requested_scene: NovaSceneId,
    pub status: AgentSceneParticipationStatus,
}

impl AgentSceneParticipation {
    pub const fn allowed(self) -> bool {
        matches!(self.status, AgentSceneParticipationStatus::Attached)
    }
}

const fn u16_budget_allows(used: u16, requested: u16, limit: u16) -> bool {
    requested <= if used >= limit { 0 } else { limit - used }
}

const fn u64_budget_allows(used: u64, requested: u64, limit: u64) -> bool {
    requested <= if used >= limit { 0 } else { limit - used }
}
