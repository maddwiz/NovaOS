use nova_fabric::{MemoryPoolKind, PlatformClass};
use nova_rt::{
    NovaAgentId, NovaIntentDispatch, NovaIntentEnvelope, NovaIntentKind, NovaPolicyDecision,
    NovaSceneDescriptor, NovaSceneId, NovaServiceId, NovaServiceStatus,
};
use novaos_acceld::AccelDispatchPlan;
use novaos_intentd::{IntentPlan, route_intent};
use novaos_memd::MemoryPlacementPlan;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShellCommand {
    Services,
    Scenes,
    MemoryPlan,
    AccelDispatch,
    SwitchScene(NovaSceneId),
    LaunchService(NovaServiceId),
    Intent(NovaIntentKind),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShellCommandParseError {
    Empty,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShellServiceStatusLine {
    pub service: NovaServiceId,
    pub name: &'static str,
    pub kind: &'static str,
    pub required: bool,
    pub state: &'static str,
    pub launch: &'static str,
    pub healthy: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShellSceneListLine {
    pub scene: NovaSceneId,
    pub name: &'static str,
    pub mode: &'static str,
    pub app_count: u16,
    pub agent_count: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShellMemoryPlacementLine {
    pub profile: &'static str,
    pub topology: &'static str,
    pub goal: &'static str,
    pub pool: &'static str,
    pub bytes: u64,
    pub status: &'static str,
    pub ready: bool,
    pub fallback: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShellAccelDispatchLine {
    pub backend: &'static str,
    pub platform: &'static str,
    pub queue: &'static str,
    pub status: &'static str,
    pub seed_ready: bool,
    pub ready: bool,
    pub cpu_fallback: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShellIntentProjectionLine {
    pub intent_id: u64,
    pub target_service: NovaServiceId,
    pub dispatch: &'static str,
    pub policy: &'static str,
    pub approval_required: bool,
    pub request_target: u64,
}

pub const fn describe_service_status(status: NovaServiceStatus) -> ShellServiceStatusLine {
    ShellServiceStatusLine {
        service: status.descriptor.id,
        name: status.descriptor.name,
        kind: status.descriptor.kind.label(),
        required: status.descriptor.required,
        state: status.state.label(),
        launch: status.last_result.status.label(),
        healthy: status.is_healthy(),
    }
}

pub const fn describe_scene(descriptor: NovaSceneDescriptor) -> ShellSceneListLine {
    ShellSceneListLine {
        scene: descriptor.id,
        name: descriptor.name,
        mode: descriptor.mode.label(),
        app_count: descriptor.app_count,
        agent_count: descriptor.agent_count,
    }
}

pub const fn describe_memory_placement(plan: MemoryPlacementPlan) -> ShellMemoryPlacementLine {
    ShellMemoryPlacementLine {
        profile: plan.profile_name,
        topology: plan.topology.label(),
        goal: plan.request.goal.label(),
        pool: memory_pool_label(plan.selected_pool),
        bytes: plan.request.bytes,
        status: plan.status.label(),
        ready: plan.is_ready(),
        fallback: plan.used_fallback(),
    }
}

pub const fn describe_accel_dispatch(plan: AccelDispatchPlan) -> ShellAccelDispatchLine {
    ShellAccelDispatchLine {
        backend: backend_name(plan),
        platform: backend_platform(plan).label(),
        queue: plan.request.queue_class.label(),
        status: plan.status.label(),
        seed_ready: plan.seed_ready,
        ready: plan.is_ready(),
        cpu_fallback: plan.used_cpu_fallback(),
    }
}

pub const fn describe_intent_plan(plan: IntentPlan) -> ShellIntentProjectionLine {
    ShellIntentProjectionLine {
        intent_id: plan.intent_id,
        target_service: plan.projection.target_service,
        dispatch: plan.projection.dispatch_label(),
        policy: plan.step.policy.label(),
        approval_required: plan.requires_approval,
        request_target: intent_request_target(plan.projection.dispatch),
    }
}

pub fn intent_for_command(
    command: ShellCommand,
    source_agent: NovaAgentId,
    scene: NovaSceneId,
    intent_id: u64,
) -> Option<NovaIntentEnvelope> {
    let (kind, target_service, policy_hint) = match command {
        ShellCommand::LaunchService(service) => (
            NovaIntentKind::LaunchService,
            service,
            NovaPolicyDecision::Ask,
        ),
        ShellCommand::SwitchScene(_) => (
            NovaIntentKind::SwitchScene,
            NovaServiceId::new(0),
            NovaPolicyDecision::Ask,
        ),
        ShellCommand::Intent(kind) => (
            kind,
            NovaServiceId::new(0),
            default_policy_hint_for_intent(kind),
        ),
        _ => return None,
    };

    Some(NovaIntentEnvelope {
        id: intent_id,
        source_agent,
        scene,
        target_service,
        kind,
        policy_hint,
    })
}

pub fn project_command(
    command: ShellCommand,
    source_agent: NovaAgentId,
    scene: NovaSceneId,
    intent_id: u64,
) -> Option<IntentPlan> {
    let intent = intent_for_command(command, source_agent, scene, intent_id)?;
    let plan = route_intent(intent);

    match command {
        ShellCommand::SwitchScene(target_scene) => Some(IntentPlan {
            projection: nova_rt::NovaIntentProjection::new(
                plan.intent_id,
                plan.projection.target_service,
                NovaIntentDispatch::SwitchScene(nova_rt::NovaSceneSwitchRequest::new(
                    source_agent,
                    scene,
                    target_scene,
                )),
            ),
            ..plan
        }),
        _ => Some(plan),
    }
}

pub fn parse_command(input: &str) -> Result<ShellCommand, ShellCommandParseError> {
    match input.trim() {
        "" => Err(ShellCommandParseError::Empty),
        "services" | "svc" => Ok(ShellCommand::Services),
        "scenes" | "scene ls" => Ok(ShellCommand::Scenes),
        "mem plan" | "memory plan" => Ok(ShellCommand::MemoryPlan),
        "accel dispatch" | "accel plan" => Ok(ShellCommand::AccelDispatch),
        "scene root" => Ok(ShellCommand::SwitchScene(NovaSceneId::ROOT)),
        "launch policyd" => launch_service(NovaServiceId::POLICYD),
        "launch agentd" => launch_service(NovaServiceId::AGENTD),
        "launch memd" => launch_service(NovaServiceId::MEMD),
        "launch acceld" => launch_service(NovaServiceId::ACCELD),
        "launch intentd" => launch_service(NovaServiceId::INTENTD),
        "launch scened" => launch_service(NovaServiceId::SCENED),
        "launch appbridged" => launch_service(NovaServiceId::APPBRIDGED),
        "launch shelld" => launch_service(NovaServiceId::SHELLD),
        "status" => Ok(ShellCommand::Intent(NovaIntentKind::RequestStatus)),
        _ => Err(ShellCommandParseError::Unknown),
    }
}

const fn launch_service(service: NovaServiceId) -> Result<ShellCommand, ShellCommandParseError> {
    Ok(ShellCommand::LaunchService(service))
}

const fn memory_pool_label(pool: Option<MemoryPoolKind>) -> &'static str {
    match pool {
        Some(pool) => pool.label(),
        None => "none",
    }
}

const fn backend_name(plan: AccelDispatchPlan) -> &'static str {
    match plan.selected_backend {
        Some(backend) => backend.name,
        None => "none",
    }
}

const fn backend_platform(plan: AccelDispatchPlan) -> PlatformClass {
    match plan.selected_backend {
        Some(backend) => backend.platform_class,
        None => PlatformClass::Unknown,
    }
}

const fn default_policy_hint_for_intent(kind: NovaIntentKind) -> NovaPolicyDecision {
    match kind {
        NovaIntentKind::RequestStatus => NovaPolicyDecision::Allow,
        NovaIntentKind::LaunchService
        | NovaIntentKind::OpenApp
        | NovaIntentKind::SwitchScene
        | NovaIntentKind::Custom => NovaPolicyDecision::Ask,
    }
}

const fn intent_request_target(dispatch: NovaIntentDispatch) -> u64 {
    match dispatch {
        NovaIntentDispatch::LaunchService(request) => request.target.0,
        NovaIntentDispatch::SwitchScene(request) => request.target_scene.0,
        NovaIntentDispatch::AppAction(request) => request.app.0,
        NovaIntentDispatch::Status(request) => request.target_service.0,
    }
}
