use crate::{
    NOVA_INIT_CAPSULE_KNOWN_CAPABILITIES_V1, NOVA_INIT_CAPSULE_SERVICE_NAME_LEN,
    NovaBootstrapTaskContextV1, NovaInitCapsuleCapabilityV1, encode_init_capsule_service_name,
};

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

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct NovaAppId(pub u64);

impl NovaAppId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct NovaTaskId(pub u64);

impl NovaTaskId {
    pub const UNASSIGNED: Self = Self(0);
    pub const BOOTSTRAP_INITD: Self = Self(1);

    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn is_assigned(self) -> bool {
        self.0 != 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct NovaEndpointId(pub u64);

impl NovaEndpointId {
    pub const UNASSIGNED: Self = Self(0);

    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn is_assigned(self) -> bool {
        self.0 != 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct NovaSharedMemoryRegionId(pub u64);

impl NovaSharedMemoryRegionId {
    pub const UNASSIGNED: Self = Self(0);

    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn is_assigned(self) -> bool {
        self.0 != 0
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

impl NovaServiceKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Kernel => "kernel",
            Self::Core => "core",
            Self::Interaction => "interaction",
            Self::Bridge => "bridge",
            Self::Operator => "operator",
        }
    }
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

impl NovaServiceState {
    pub const fn label(self) -> &'static str {
        match self {
            Self::NotStarted => "not-started",
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Degraded => "degraded",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
        }
    }
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
    pub last_result: NovaServiceLaunchResult,
}

impl NovaServiceStatus {
    pub const fn new(
        descriptor: NovaServiceDescriptor,
        state: NovaServiceState,
        last_result: NovaServiceLaunchResult,
    ) -> Self {
        Self {
            descriptor,
            state,
            last_result,
        }
    }

    pub const fn running(descriptor: NovaServiceDescriptor) -> Self {
        Self::new(
            descriptor,
            NovaServiceState::Running,
            NovaServiceLaunchResult::started(descriptor.id),
        )
    }

    pub const fn deferred(descriptor: NovaServiceDescriptor, detail: u64) -> Self {
        Self::new(
            descriptor,
            NovaServiceState::NotStarted,
            NovaServiceLaunchResult::deferred(descriptor.id, detail),
        )
    }

    pub const fn is_healthy(self) -> bool {
        matches!(
            self.state,
            NovaServiceState::Running | NovaServiceState::Degraded
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NovaServiceLaunchRequest {
    pub requester: NovaServiceId,
    pub target: NovaServiceId,
    pub scene: NovaSceneId,
    pub flags: u64,
}

impl NovaServiceLaunchRequest {
    pub const fn new(
        requester: NovaServiceId,
        target: NovaServiceId,
        scene: NovaSceneId,
        flags: u64,
    ) -> Self {
        Self {
            requester,
            target,
            scene,
            flags,
        }
    }
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

impl NovaServiceLaunchStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::NotRequested => "not-requested",
            Self::Started => "started",
            Self::AlreadyRunning => "already-running",
            Self::Deferred => "deferred",
            Self::Denied => "denied",
            Self::Failed => "failed",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NovaServiceLaunchResult {
    pub target: NovaServiceId,
    pub status: NovaServiceLaunchStatus,
    pub detail: u64,
}

impl NovaServiceLaunchResult {
    pub const fn new(target: NovaServiceId, status: NovaServiceLaunchStatus, detail: u64) -> Self {
        Self {
            target,
            status,
            detail,
        }
    }

    pub const fn started(target: NovaServiceId) -> Self {
        Self::new(target, NovaServiceLaunchStatus::Started, 0)
    }

    pub const fn deferred(target: NovaServiceId, detail: u64) -> Self {
        Self::new(target, NovaServiceLaunchStatus::Deferred, detail)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NovaServiceBootstrapRequirement {
    pub requested_capabilities: u64,
    pub endpoint_slots: u32,
    pub shared_memory_regions: u32,
}

impl NovaServiceBootstrapRequirement {
    pub const fn new(
        requested_capabilities: u64,
        endpoint_slots: u32,
        shared_memory_regions: u32,
    ) -> Self {
        Self {
            requested_capabilities,
            endpoint_slots,
            shared_memory_regions,
        }
    }

    pub const fn core_required() -> Self {
        Self::new(
            NovaInitCapsuleCapabilityV1::BootLog as u64
                | NovaInitCapsuleCapabilityV1::Yield as u64
                | NovaInitCapsuleCapabilityV1::EndpointBootstrap as u64
                | NovaInitCapsuleCapabilityV1::SharedMemoryBootstrap as u64,
            1,
            1,
        )
    }

    pub const fn boot_log_only() -> Self {
        Self::new(NovaInitCapsuleCapabilityV1::BootLog as u64, 0, 0)
    }

    pub const fn is_valid(self) -> bool {
        let known_caps =
            (self.requested_capabilities & !NOVA_INIT_CAPSULE_KNOWN_CAPABILITIES_V1) == 0;
        let endpoint_cap = self.has_capability(NovaInitCapsuleCapabilityV1::EndpointBootstrap);
        let shared_memory_cap =
            self.has_capability(NovaInitCapsuleCapabilityV1::SharedMemoryBootstrap);
        known_caps
            && ((self.endpoint_slots == 0 && !endpoint_cap)
                || (self.endpoint_slots != 0 && endpoint_cap))
            && ((self.shared_memory_regions == 0 && !shared_memory_cap)
                || (self.shared_memory_regions != 0 && shared_memory_cap))
    }

    pub const fn has_capability(self, capability: NovaInitCapsuleCapabilityV1) -> bool {
        (self.requested_capabilities & capability as u64) != 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NovaServiceLaunchSpec {
    pub descriptor: NovaServiceDescriptor,
    pub bootstrap: NovaServiceBootstrapRequirement,
}

impl NovaServiceLaunchSpec {
    pub const fn new(
        descriptor: NovaServiceDescriptor,
        bootstrap: NovaServiceBootstrapRequirement,
    ) -> Self {
        Self {
            descriptor,
            bootstrap,
        }
    }

    pub fn is_valid(self) -> bool {
        !self.descriptor.id.is_empty()
            && self.bootstrap.is_valid()
            && self.encoded_service_name().is_some()
    }

    pub const fn launch_request(
        self,
        requester: NovaServiceId,
        scene: NovaSceneId,
    ) -> NovaServiceLaunchRequest {
        NovaServiceLaunchRequest::new(requester, self.descriptor.id, scene, 0)
    }

    pub fn encoded_service_name(self) -> Option<[u8; NOVA_INIT_CAPSULE_SERVICE_NAME_LEN]> {
        encode_init_capsule_service_name(self.descriptor.name)
    }

    pub fn bootstrap_context_v1(self) -> Option<NovaBootstrapTaskContextV1> {
        Some(NovaBootstrapTaskContextV1::new(
            self.encoded_service_name()?,
            self.bootstrap.requested_capabilities,
            self.bootstrap.endpoint_slots,
            self.bootstrap.shared_memory_regions,
        ))
        .filter(|context| context.is_valid())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum NovaServiceBindingState {
    ModelOnly = 0,
    Planned = 1,
    KernelTaskReady = 2,
    EndpointReady = 3,
    SharedMemoryReady = 4,
    KernelBacked = 5,
}

impl NovaServiceBindingState {
    pub const fn label(self) -> &'static str {
        match self {
            Self::ModelOnly => "model-only",
            Self::Planned => "planned",
            Self::KernelTaskReady => "kernel-task-ready",
            Self::EndpointReady => "endpoint-ready",
            Self::SharedMemoryReady => "shared-memory-ready",
            Self::KernelBacked => "kernel-backed",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NovaServiceKernelBinding {
    pub service: NovaServiceId,
    pub task: NovaTaskId,
    pub control_endpoint: NovaEndpointId,
    pub shared_memory_region: NovaSharedMemoryRegionId,
    pub state: NovaServiceBindingState,
    pub health_generation: u64,
}

impl NovaServiceKernelBinding {
    pub const fn model_only(service: NovaServiceId) -> Self {
        Self {
            service,
            task: NovaTaskId::UNASSIGNED,
            control_endpoint: NovaEndpointId::UNASSIGNED,
            shared_memory_region: NovaSharedMemoryRegionId::UNASSIGNED,
            state: NovaServiceBindingState::ModelOnly,
            health_generation: 0,
        }
    }

    pub const fn planned(
        service: NovaServiceId,
        task: NovaTaskId,
        control_endpoint: NovaEndpointId,
        shared_memory_region: NovaSharedMemoryRegionId,
    ) -> Self {
        Self {
            service,
            task,
            control_endpoint,
            shared_memory_region,
            state: NovaServiceBindingState::Planned,
            health_generation: 0,
        }
    }

    pub const fn kernel_backed(
        service: NovaServiceId,
        task: NovaTaskId,
        control_endpoint: NovaEndpointId,
        shared_memory_region: NovaSharedMemoryRegionId,
        health_generation: u64,
    ) -> Self {
        Self {
            service,
            task,
            control_endpoint,
            shared_memory_region,
            state: NovaServiceBindingState::KernelBacked,
            health_generation,
        }
    }

    pub const fn has_kernel_objects(self) -> bool {
        self.task.is_assigned()
            && self.control_endpoint.is_assigned()
            && self.shared_memory_region.is_assigned()
    }

    pub const fn can_publish_kernel_health(self) -> bool {
        self.has_kernel_objects()
            && matches!(self.state, NovaServiceBindingState::KernelBacked)
            && self.health_generation != 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NovaServiceKernelLaunchPlan {
    pub descriptor: NovaServiceDescriptor,
    pub request: NovaServiceLaunchRequest,
    pub binding: NovaServiceKernelBinding,
}

impl NovaServiceKernelLaunchPlan {
    pub const fn new(
        descriptor: NovaServiceDescriptor,
        request: NovaServiceLaunchRequest,
        binding: NovaServiceKernelBinding,
    ) -> Self {
        Self {
            descriptor,
            request,
            binding,
        }
    }

    pub const fn requires_kernel_launch(self) -> bool {
        self.descriptor.required && !self.binding.can_publish_kernel_health()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum NovaPolicyDecision {
    Allow = 1,
    Deny = 2,
    Ask = 3,
}

impl NovaPolicyDecision {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Deny => "deny",
            Self::Ask => "ask",
        }
    }
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
    Service(NovaServiceId),
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

impl NovaIntentKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::LaunchService => "launch-service",
            Self::OpenApp => "open-app",
            Self::SwitchScene => "switch-scene",
            Self::RequestStatus => "request-status",
            Self::Custom => "custom",
        }
    }
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
pub struct NovaSceneSwitchRequest {
    pub requested_by: NovaAgentId,
    pub current_scene: NovaSceneId,
    pub target_scene: NovaSceneId,
}

impl NovaSceneSwitchRequest {
    pub const fn new(
        requested_by: NovaAgentId,
        current_scene: NovaSceneId,
        target_scene: NovaSceneId,
    ) -> Self {
        Self {
            requested_by,
            current_scene,
            target_scene,
        }
    }

    pub const fn unresolved(requested_by: NovaAgentId, current_scene: NovaSceneId) -> Self {
        Self::new(requested_by, current_scene, NovaSceneId::new(0))
    }

    pub const fn has_target(self) -> bool {
        !self.target_scene.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NovaAppActionRequest {
    pub app: NovaAppId,
    pub scene: NovaSceneId,
    pub requested_by: NovaAgentId,
    pub action: NovaAppActionKind,
}

impl NovaAppActionRequest {
    pub const fn new(
        app: NovaAppId,
        scene: NovaSceneId,
        requested_by: NovaAgentId,
        action: NovaAppActionKind,
    ) -> Self {
        Self {
            app,
            scene,
            requested_by,
            action,
        }
    }

    pub const fn unresolved(
        scene: NovaSceneId,
        requested_by: NovaAgentId,
        action: NovaAppActionKind,
    ) -> Self {
        Self::new(NovaAppId::new(0), scene, requested_by, action)
    }

    pub const fn has_target_app(self) -> bool {
        !self.app.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NovaStatusRequest {
    pub requested_by: NovaAgentId,
    pub scene: NovaSceneId,
    pub target_service: NovaServiceId,
}

impl NovaStatusRequest {
    pub const fn new(
        requested_by: NovaAgentId,
        scene: NovaSceneId,
        target_service: NovaServiceId,
    ) -> Self {
        Self {
            requested_by,
            scene,
            target_service,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NovaIntentDispatch {
    LaunchService(NovaServiceLaunchRequest),
    SwitchScene(NovaSceneSwitchRequest),
    AppAction(NovaAppActionRequest),
    Status(NovaStatusRequest),
}

impl NovaIntentDispatch {
    pub const fn label(self) -> &'static str {
        match self {
            Self::LaunchService(_) => "launch-service",
            Self::SwitchScene(_) => "switch-scene",
            Self::AppAction(_) => "app-action",
            Self::Status(_) => "status",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NovaIntentProjection {
    pub intent_id: u64,
    pub target_service: NovaServiceId,
    pub dispatch: NovaIntentDispatch,
}

impl NovaIntentProjection {
    pub const fn new(
        intent_id: u64,
        target_service: NovaServiceId,
        dispatch: NovaIntentDispatch,
    ) -> Self {
        Self {
            intent_id,
            target_service,
            dispatch,
        }
    }

    pub const fn dispatch_label(self) -> &'static str {
        self.dispatch.label()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum NovaSceneMode {
    Consumer = 1,
    Pro = 2,
    Operator = 3,
}

impl NovaSceneMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Consumer => "consumer",
            Self::Pro => "pro",
            Self::Operator => "operator",
        }
    }
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

impl NovaAppBridgeKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Compatibility => "compatibility",
            Self::Remote => "remote",
        }
    }
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

impl NovaAppActionKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Launch => "launch",
            Self::Open => "open",
            Self::Focus => "focus",
            Self::Close => "close",
            Self::RequestAction => "request-action",
        }
    }
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
        NovaAppActionKind, NovaAppActionRequest, NovaAppBridgeKind, NovaEndpointId,
        NovaIntentDispatch, NovaIntentKind, NovaIntentProjection, NovaPolicyAction,
        NovaPolicyDecision, NovaPolicyRequest, NovaPolicyScope, NovaSceneMode,
        NovaSceneSwitchRequest, NovaServiceBindingState, NovaServiceBootstrapRequirement,
        NovaServiceId, NovaServiceKernelBinding, NovaServiceKernelLaunchPlan, NovaServiceKind,
        NovaServiceLaunchRequest, NovaServiceLaunchResult, NovaServiceLaunchSpec,
        NovaServiceLaunchStatus, NovaServiceStatus, NovaSharedMemoryRegionId, NovaStatusRequest,
        NovaTaskId,
    };

    #[test]
    fn service_ids_cover_core_runtime_graph() {
        assert_ne!(NovaServiceId::INITD, NovaServiceId::POLICYD);
        assert_ne!(NovaServiceId::AGENTD, NovaServiceId::INTENTD);
        assert_eq!(NovaServiceKind::Core as u16, 2);
    }

    #[test]
    fn service_enums_expose_stable_labels() {
        assert_eq!(NovaServiceKind::Interaction.label(), "interaction");
        assert_eq!(super::NovaServiceState::Running.label(), "running");
        assert_eq!(
            NovaServiceLaunchStatus::AlreadyRunning.label(),
            "already-running"
        );
        assert_eq!(
            NovaServiceBindingState::KernelBacked.label(),
            "kernel-backed"
        );
        assert_eq!(NovaPolicyDecision::Allow.label(), "allow");
        assert_eq!(NovaIntentKind::RequestStatus.label(), "request-status");
        assert_eq!(NovaSceneMode::Operator.label(), "operator");
        assert_eq!(NovaAppBridgeKind::Compatibility.label(), "compatibility");
        assert_eq!(NovaAppActionKind::RequestAction.label(), "request-action");
    }

    #[test]
    fn launch_result_reports_started_target() {
        let result = NovaServiceLaunchResult::started(NovaServiceId::POLICYD);
        assert_eq!(result.target, NovaServiceId::POLICYD);
        assert_eq!(result.status, NovaServiceLaunchStatus::Started);
    }

    #[test]
    fn service_launch_spec_rejects_unknown_bootstrap_capability_bits() {
        let descriptor = super::NovaServiceDescriptor::new(
            NovaServiceId::POLICYD,
            "policyd",
            NovaServiceKind::Core,
            true,
            10,
        );
        let spec = NovaServiceLaunchSpec::new(
            descriptor,
            NovaServiceBootstrapRequirement::new(
                super::NOVA_INIT_CAPSULE_KNOWN_CAPABILITIES_V1 | (1 << 20),
                0,
                0,
            ),
        );

        assert!(!spec.is_valid());
    }

    #[test]
    fn service_bootstrap_requirement_presets_are_valid() {
        let required = NovaServiceBootstrapRequirement::core_required();
        let boot_log_only = NovaServiceBootstrapRequirement::boot_log_only();

        assert!(required.is_valid());
        assert!(boot_log_only.is_valid());
        assert_eq!(required.endpoint_slots, 1);
        assert_eq!(boot_log_only.endpoint_slots, 0);
    }

    #[test]
    fn service_launch_spec_encodes_init_capsule_service_name() {
        let descriptor = super::NovaServiceDescriptor::new(
            NovaServiceId::POLICYD,
            "policyd",
            NovaServiceKind::Core,
            true,
            10,
        );
        let spec = NovaServiceLaunchSpec::new(
            descriptor,
            NovaServiceBootstrapRequirement::new(
                super::NovaInitCapsuleCapabilityV1::BootLog as u64,
                0,
                0,
            ),
        );

        assert_eq!(
            spec.encoded_service_name().expect("service name")[..7],
            *b"policyd"
        );
    }

    #[test]
    fn service_launch_spec_builds_bootstrap_context() {
        let descriptor = super::NovaServiceDescriptor::new(
            NovaServiceId::AGENTD,
            "agentd",
            NovaServiceKind::Core,
            true,
            20,
        );
        let caps = super::NovaInitCapsuleCapabilityV1::BootLog as u64
            | super::NovaInitCapsuleCapabilityV1::EndpointBootstrap as u64
            | super::NovaInitCapsuleCapabilityV1::SharedMemoryBootstrap as u64;
        let spec = NovaServiceLaunchSpec::new(
            descriptor,
            NovaServiceBootstrapRequirement::new(caps, 1, 1),
        );
        let context = spec.bootstrap_context_v1().expect("context");

        assert!(context.is_valid());
        assert_eq!(context.service_name(), "agentd");
        assert_eq!(context.endpoint_slots, 1);
        assert_eq!(context.shared_memory_regions, 1);
    }

    #[test]
    fn service_status_tracks_health_from_launch_result() {
        let descriptor = super::NovaServiceDescriptor::new(
            NovaServiceId::INTENTD,
            "intentd",
            NovaServiceKind::Interaction,
            true,
            50,
        );
        let running = NovaServiceStatus::running(descriptor);
        let deferred = NovaServiceStatus::deferred(descriptor, 7);

        assert!(running.is_healthy());
        assert!(!deferred.is_healthy());
        assert_eq!(
            deferred.last_result.status,
            NovaServiceLaunchStatus::Deferred
        );
        assert_eq!(deferred.last_result.detail, 7);
    }

    #[test]
    fn service_kernel_binding_tracks_planned_and_backed_states() {
        let planned = NovaServiceKernelBinding::planned(
            NovaServiceId::POLICYD,
            NovaTaskId::new(0x1001),
            NovaEndpointId::new(0x2001),
            NovaSharedMemoryRegionId::new(0x3001),
        );
        let backed = NovaServiceKernelBinding::kernel_backed(
            NovaServiceId::POLICYD,
            planned.task,
            planned.control_endpoint,
            planned.shared_memory_region,
            9,
        );

        assert_eq!(planned.state, NovaServiceBindingState::Planned);
        assert!(planned.has_kernel_objects());
        assert!(!planned.can_publish_kernel_health());
        assert!(backed.can_publish_kernel_health());
    }

    #[test]
    fn service_kernel_launch_plan_requires_kernel_until_backed() {
        let descriptor = super::NovaServiceDescriptor::new(
            NovaServiceId::AGENTD,
            "agentd",
            NovaServiceKind::Core,
            true,
            20,
        );
        let request = NovaServiceLaunchRequest::new(
            NovaServiceId::INITD,
            NovaServiceId::AGENTD,
            super::NovaSceneId::ROOT,
            0,
        );
        let planned = NovaServiceKernelLaunchPlan::new(
            descriptor,
            request,
            NovaServiceKernelBinding::planned(
                NovaServiceId::AGENTD,
                NovaTaskId::new(0x1002),
                NovaEndpointId::new(0x2002),
                NovaSharedMemoryRegionId::new(0x3002),
            ),
        );
        let backed = NovaServiceKernelLaunchPlan::new(
            descriptor,
            request,
            NovaServiceKernelBinding::kernel_backed(
                NovaServiceId::AGENTD,
                NovaTaskId::new(0x1002),
                NovaEndpointId::new(0x2002),
                NovaSharedMemoryRegionId::new(0x3002),
                1,
            ),
        );

        assert!(planned.requires_kernel_launch());
        assert!(!backed.requires_kernel_launch());
    }

    #[test]
    fn policy_request_can_target_service_or_bridge_scope() {
        let request = NovaPolicyRequest {
            subject_service: NovaServiceId::AGENTD,
            subject_agent: super::NovaAgentId::INIT,
            action: NovaPolicyAction::AppAction,
            scope: NovaPolicyScope::Service(NovaServiceId::APPBRIDGED),
        };

        assert_eq!(request.action, NovaPolicyAction::AppAction);
        assert_eq!(
            request.scope,
            NovaPolicyScope::Service(NovaServiceId::APPBRIDGED)
        );
        assert_eq!(NovaPolicyDecision::Ask as u16, 3);
    }

    #[test]
    fn intent_projection_requests_track_empty_and_resolved_targets() {
        let unresolved_scene =
            NovaSceneSwitchRequest::unresolved(super::NovaAgentId::INIT, super::NovaSceneId::ROOT);
        let unresolved_app = NovaAppActionRequest::unresolved(
            super::NovaSceneId::ROOT,
            super::NovaAgentId::INIT,
            NovaAppActionKind::Open,
        );
        let status = NovaStatusRequest::new(
            super::NovaAgentId::INIT,
            super::NovaSceneId::ROOT,
            NovaServiceId::SHELLD,
        );

        assert!(!unresolved_scene.has_target());
        assert!(!unresolved_app.has_target_app());
        assert_eq!(status.target_service, NovaServiceId::SHELLD);
    }

    #[test]
    fn intent_projection_carries_typed_dispatch_payloads() {
        let launch = NovaServiceLaunchRequest::new(
            NovaServiceId::INTENTD,
            NovaServiceId::AGENTD,
            super::NovaSceneId::ROOT,
            0,
        );
        let projection = NovaIntentProjection::new(
            7,
            NovaServiceId::AGENTD,
            NovaIntentDispatch::LaunchService(launch),
        );

        assert_eq!(projection.intent_id, 7);
        assert_eq!(projection.target_service, NovaServiceId::AGENTD);
        assert_eq!(projection.dispatch_label(), "launch-service");
        assert_eq!(
            projection.dispatch,
            NovaIntentDispatch::LaunchService(launch)
        );
    }
}
