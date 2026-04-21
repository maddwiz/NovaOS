#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct NovaServiceId(pub u64);

impl NovaServiceId {
    pub const INITD: Self = Self(0x494E_4954_445F_3031);
    pub const POLICYD: Self = Self(0x504F_4C49_4359_4431);
    pub const AGENTD: Self = Self(0x4147_454E_5444_3031);
    pub const MEMD: Self = Self(0x4D45_4D44_5F5F_3031);
    pub const ACCELD: Self = Self(0x4143_4345_4C44_3031);
    pub const INTENTD: Self = Self(0x494E_5445_4E54_4431);
    pub const SCENED: Self = Self(0x5343_454E_4544_3031);
    pub const APPBRIDGED: Self = Self(0x4150_5042_5247_3031);
    pub const SHELLD: Self = Self(0x5348_454C_4C44_3031);

    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct NovaAgentId(pub u64);

impl NovaAgentId {
    pub const KERNEL: Self = Self(1);
    pub const INIT: Self = Self(2);

    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct NovaSceneId(pub u64);

impl NovaSceneId {
    pub const ROOT: Self = Self(1);

    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct NovaAppId(pub u64);

impl NovaAppId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum NovaServiceKind {
    Kernel = 1,
    Core = 2,
    Interaction = 3,
    Bridge = 4,
    Operator = 5,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum NovaServiceState {
    NotStarted = 0,
    Starting = 1,
    Running = 2,
    Degraded = 3,
    Stopped = 4,
    Failed = 5,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NovaServiceDescriptor {
    pub id: NovaServiceId,
    pub name: &'static str,
    pub kind: NovaServiceKind,
    pub required: bool,
    pub launch_order: u16,
}

impl NovaServiceDescriptor {
    pub const fn new(
        id: NovaServiceId,
        name: &'static str,
        kind: NovaServiceKind,
        required: bool,
        launch_order: u16,
    ) -> Self {
        Self {
            id,
            name,
            kind,
            required,
            launch_order,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NovaServiceStatus {
    pub descriptor: NovaServiceDescriptor,
    pub state: NovaServiceState,
    pub last_result: NovaServiceLaunchStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NovaServiceLaunchRequest {
    pub requester: NovaServiceId,
    pub target: NovaServiceId,
    pub scene: NovaSceneId,
    pub flags: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum NovaServiceLaunchStatus {
    NotRequested = 0,
    Started = 1,
    AlreadyRunning = 2,
    Deferred = 3,
    Denied = 4,
    Failed = 5,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NovaServiceLaunchResult {
    pub target: NovaServiceId,
    pub status: NovaServiceLaunchStatus,
    pub detail: u64,
}

impl NovaServiceLaunchResult {
    pub const fn started(target: NovaServiceId) -> Self {
        Self {
            target,
            status: NovaServiceLaunchStatus::Started,
            detail: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum NovaPolicyDecision {
    Allow = 1,
    Deny = 2,
    Ask = 3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum NovaPolicyAction {
    LaunchService = 1,
    StopService = 2,
    RouteIntent = 3,
    AccessMemory = 4,
    AppAction = 5,
    DelegateToAgent = 6,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NovaPolicyScope {
    System,
    Scene(NovaSceneId),
    Agent(NovaAgentId),
    App(NovaAppId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NovaPolicyRequest {
    pub subject_service: NovaServiceId,
    pub subject_agent: NovaAgentId,
    pub action: NovaPolicyAction,
    pub scope: NovaPolicyScope,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum NovaIntentKind {
    LaunchService = 1,
    OpenApp = 2,
    SwitchScene = 3,
    RequestStatus = 4,
    Custom = 0xffff,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NovaIntentEnvelope {
    pub id: u64,
    pub source_agent: NovaAgentId,
    pub scene: NovaSceneId,
    pub target_service: NovaServiceId,
    pub kind: NovaIntentKind,
    pub policy_hint: NovaPolicyDecision,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum NovaSceneMode {
    Consumer = 1,
    Pro = 2,
    Operator = 3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NovaSceneDescriptor {
    pub id: NovaSceneId,
    pub name: &'static str,
    pub mode: NovaSceneMode,
    pub owner_agent: NovaAgentId,
    pub app_count: u16,
    pub agent_count: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum NovaAppBridgeKind {
    Native = 1,
    Compatibility = 2,
    Remote = 3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum NovaAppActionKind {
    Launch = 1,
    Open = 2,
    Focus = 3,
    Close = 4,
    RequestAction = 5,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NovaAppDescriptor {
    pub id: NovaAppId,
    pub name: &'static str,
    pub bridge: NovaAppBridgeKind,
    pub action_count: u16,
}

#[cfg(test)]
mod tests {
    use super::{
        NovaPolicyAction, NovaPolicyDecision, NovaPolicyRequest, NovaPolicyScope, NovaServiceId,
        NovaServiceKind, NovaServiceLaunchResult, NovaServiceLaunchStatus,
    };

    #[test]
    fn service_ids_cover_core_runtime_graph() {
        assert_ne!(NovaServiceId::INITD, NovaServiceId::POLICYD);
        assert_ne!(NovaServiceId::AGENTD, NovaServiceId::INTENTD);
        assert_eq!(NovaServiceKind::Core as u16, 2);
    }

    #[test]
    fn launch_result_reports_started_target() {
        let result = NovaServiceLaunchResult::started(NovaServiceId::POLICYD);
        assert_eq!(result.target, NovaServiceId::POLICYD);
        assert_eq!(result.status, NovaServiceLaunchStatus::Started);
    }

    #[test]
    fn policy_request_can_target_system_or_bridge_scope() {
        let request = NovaPolicyRequest {
            subject_service: NovaServiceId::AGENTD,
            subject_agent: super::NovaAgentId::INIT,
            action: NovaPolicyAction::AppAction,
            scope: NovaPolicyScope::System,
        };

        assert_eq!(request.action, NovaPolicyAction::AppAction);
        assert_eq!(NovaPolicyDecision::Ask as u16, 3);
    }
}
