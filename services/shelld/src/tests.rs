use crate::{
    SHELLD_LAUNCH_SPEC, ShellCommand, ShellCommandParseError, describe_accel_dispatch,
    describe_intent_plan, describe_memory_placement, describe_scene, describe_service_status,
    intent_for_command, parse_command, project_command,
};
use nova_fabric::{MemoryTopologyClass, QueueClass};
use nova_rt::{
    NovaAgentId, NovaIntentDispatch, NovaIntentKind, NovaPolicyDecision, NovaSceneDescriptor,
    NovaSceneId, NovaSceneMode, NovaServiceDescriptor, NovaServiceId, NovaServiceKind,
    NovaServiceLaunchResult, NovaServiceLaunchStatus, NovaServiceState, NovaServiceStatus,
};
use novaos_acceld::{
    AccelDispatchPlan, AccelDispatchRequest, AccelDispatchStatus, BackendDescriptor,
};
use novaos_memd::{
    MemoryPlacementGoal, MemoryPlacementPlan, MemoryPlacementRequest, MemoryPlacementStatus,
};

#[test]
fn parser_lists_services() {
    assert_eq!(parse_command("services"), Ok(ShellCommand::Services));
}

#[test]
fn parser_exposes_memory_and_accel_operator_commands() {
    assert_eq!(parse_command("mem plan"), Ok(ShellCommand::MemoryPlan));
    assert_eq!(parse_command("memory plan"), Ok(ShellCommand::MemoryPlan));
    assert_eq!(
        parse_command("accel dispatch"),
        Ok(ShellCommand::AccelDispatch)
    );
    assert_eq!(parse_command("accel plan"), Ok(ShellCommand::AccelDispatch));
}

#[test]
fn launch_spec_identifies_optional_shell_service() {
    assert_eq!(SHELLD_LAUNCH_SPEC.descriptor.id, NovaServiceId::SHELLD);
    assert!(!SHELLD_LAUNCH_SPEC.descriptor.required);
    assert!(SHELLD_LAUNCH_SPEC.is_valid());
}

#[test]
fn parser_launches_core_services_by_name() {
    assert_eq!(
        parse_command("launch policyd"),
        Ok(ShellCommand::LaunchService(NovaServiceId::POLICYD))
    );
    assert_eq!(
        parse_command("launch memd"),
        Ok(ShellCommand::LaunchService(NovaServiceId::MEMD))
    );
    assert_eq!(
        parse_command("launch acceld"),
        Ok(ShellCommand::LaunchService(NovaServiceId::ACCELD))
    );
}

#[test]
fn parser_launches_interaction_and_bridge_services_by_name() {
    assert_eq!(
        parse_command("launch intentd"),
        Ok(ShellCommand::LaunchService(NovaServiceId::INTENTD))
    );
    assert_eq!(
        parse_command("launch scened"),
        Ok(ShellCommand::LaunchService(NovaServiceId::SCENED))
    );
    assert_eq!(
        parse_command("launch appbridged"),
        Ok(ShellCommand::LaunchService(NovaServiceId::APPBRIDGED))
    );
    assert_eq!(
        parse_command("launch shelld"),
        Ok(ShellCommand::LaunchService(NovaServiceId::SHELLD))
    );
}

#[test]
fn shell_launch_command_projects_launch_service_intent() {
    let plan = project_command(
        ShellCommand::LaunchService(NovaServiceId::MEMD),
        NovaAgentId::INIT,
        NovaSceneId::ROOT,
        11,
    )
    .expect("launch intent");
    let line = describe_intent_plan(plan);

    assert_eq!(plan.primary_service, NovaServiceId::SHELLD);
    assert_eq!(line.intent_id, 11);
    assert_eq!(line.target_service, NovaServiceId::SHELLD);
    assert_eq!(line.dispatch, "launch-service");
    assert_eq!(line.policy, "ask");
    assert!(line.approval_required);
    assert_eq!(line.request_target, NovaServiceId::MEMD.0);
    match plan.projection.dispatch {
        NovaIntentDispatch::LaunchService(request) => {
            assert_eq!(request.requester, NovaServiceId::INTENTD);
            assert_eq!(request.target, NovaServiceId::MEMD);
        }
        dispatch => panic!("unexpected dispatch: {dispatch:?}"),
    }
}

#[test]
fn shell_scene_command_projects_resolved_scene_switch_request() {
    let plan = project_command(
        ShellCommand::SwitchScene(NovaSceneId::ROOT),
        NovaAgentId::new(7),
        NovaSceneId::new(9),
        12,
    )
    .expect("scene intent");
    let line = describe_intent_plan(plan);

    assert_eq!(line.dispatch, "switch-scene");
    assert_eq!(line.policy, "ask");
    assert!(line.approval_required);
    assert_eq!(line.request_target, NovaSceneId::ROOT.0);
    match plan.projection.dispatch {
        NovaIntentDispatch::SwitchScene(request) => {
            assert_eq!(request.requested_by, NovaAgentId::new(7));
            assert_eq!(request.current_scene, NovaSceneId::new(9));
            assert_eq!(request.target_scene, NovaSceneId::ROOT);
            assert!(request.has_target());
        }
        dispatch => panic!("unexpected dispatch: {dispatch:?}"),
    }
}

#[test]
fn shell_status_command_projects_allow_status_intent() {
    let command = ShellCommand::Intent(NovaIntentKind::RequestStatus);
    let intent = intent_for_command(command, NovaAgentId::INIT, NovaSceneId::ROOT, 13)
        .expect("status envelope");
    let plan =
        project_command(command, NovaAgentId::INIT, NovaSceneId::ROOT, 13).expect("status plan");
    let line = describe_intent_plan(plan);

    assert_eq!(intent.kind, NovaIntentKind::RequestStatus);
    assert_eq!(intent.policy_hint, NovaPolicyDecision::Allow);
    assert_eq!(line.dispatch, "status");
    assert_eq!(line.policy, "allow");
    assert!(!line.approval_required);
    assert_eq!(line.request_target, NovaServiceId::SHELLD.0);
}

#[test]
fn service_status_view_exposes_stable_operator_fields() {
    let descriptor = NovaServiceDescriptor::new(
        NovaServiceId::POLICYD,
        "policyd",
        NovaServiceKind::Core,
        true,
        10,
    );
    let status = NovaServiceStatus::new(
        descriptor,
        NovaServiceState::Running,
        NovaServiceLaunchResult::started(NovaServiceId::POLICYD),
    );
    let line = describe_service_status(status);

    assert_eq!(line.service, NovaServiceId::POLICYD);
    assert_eq!(line.name, "policyd");
    assert_eq!(line.kind, "core");
    assert!(line.required);
    assert_eq!(line.state, "running");
    assert_eq!(line.launch, "started");
    assert!(line.healthy);
}

#[test]
fn service_status_view_marks_deferred_services_unhealthy() {
    let descriptor = NovaServiceDescriptor::new(
        NovaServiceId::SHELLD,
        "shelld",
        NovaServiceKind::Operator,
        false,
        80,
    );
    let status = NovaServiceStatus::new(
        descriptor,
        NovaServiceState::NotStarted,
        NovaServiceLaunchResult::new(NovaServiceId::SHELLD, NovaServiceLaunchStatus::Deferred, 1),
    );
    let line = describe_service_status(status);

    assert_eq!(line.name, "shelld");
    assert_eq!(line.kind, "operator");
    assert!(!line.required);
    assert_eq!(line.state, "not-started");
    assert_eq!(line.launch, "deferred");
    assert!(!line.healthy);
}

#[test]
fn scene_view_exposes_stable_operator_fields() {
    let descriptor = NovaSceneDescriptor {
        id: NovaSceneId::ROOT,
        name: "root",
        mode: NovaSceneMode::Operator,
        owner_agent: NovaAgentId::INIT,
        app_count: 2,
        agent_count: 3,
    };
    let line = describe_scene(descriptor);

    assert_eq!(line.scene, NovaSceneId::ROOT);
    assert_eq!(line.name, "root");
    assert_eq!(line.mode, "operator");
    assert_eq!(line.app_count, 2);
    assert_eq!(line.agent_count, 3);
}

#[test]
fn memory_placement_view_exposes_stable_operator_fields() {
    let plan = MemoryPlacementPlan::new(
        "uma",
        MemoryTopologyClass::Uma,
        MemoryPlacementRequest::exact(4096, MemoryPlacementGoal::AcceleratorVisible),
        Some(nova_fabric::MemoryPoolKind::UmaAccelVisible),
        MemoryPlacementStatus::Ready,
    );
    let line = describe_memory_placement(plan);

    assert_eq!(line.profile, "uma");
    assert_eq!(line.topology, "uma");
    assert_eq!(line.goal, "accelerator-visible");
    assert_eq!(line.pool, "uma-accel-visible");
    assert_eq!(line.bytes, 4096);
    assert_eq!(line.status, "ready");
    assert!(line.ready);
    assert!(!line.fallback);
}

#[test]
fn memory_placement_view_marks_missing_pool_as_none() {
    let plan = MemoryPlacementPlan::new(
        "discrete",
        MemoryTopologyClass::Discrete,
        MemoryPlacementRequest::exact(8192, MemoryPlacementGoal::PeerFabric),
        None,
        MemoryPlacementStatus::UnsupportedGoal,
    );
    let line = describe_memory_placement(plan);

    assert_eq!(line.profile, "discrete");
    assert_eq!(line.pool, "none");
    assert_eq!(line.status, "unsupported-goal");
    assert!(!line.ready);
    assert!(!line.fallback);
}

#[test]
fn accel_dispatch_view_exposes_stable_operator_fields() {
    let backend = BackendDescriptor {
        name: "gb10",
        platform_class: nova_fabric::PlatformClass::SparkUma,
        capability_flags: nova_fabric::FabricCapabilityFlags::INTEGRATED_ACCEL,
        queue_class_count: 4,
    };
    let plan = AccelDispatchPlan::new(
        AccelDispatchRequest::exact(QueueClass::Latency),
        true,
        Some(backend),
        AccelDispatchStatus::Ready,
    );
    let line = describe_accel_dispatch(plan);

    assert_eq!(line.backend, "gb10");
    assert_eq!(line.platform, "spark-uma");
    assert_eq!(line.queue, "latency");
    assert_eq!(line.status, "ready");
    assert!(line.seed_ready);
    assert!(line.ready);
    assert!(!line.cpu_fallback);
}

#[test]
fn accel_dispatch_view_marks_missing_backend_as_none() {
    let plan = AccelDispatchPlan::new(
        AccelDispatchRequest::new(QueueClass::LowPriBackground, true),
        false,
        None,
        AccelDispatchStatus::MissingPlatformSeed,
    );
    let line = describe_accel_dispatch(plan);

    assert_eq!(line.backend, "none");
    assert_eq!(line.platform, "unknown");
    assert_eq!(line.queue, "low-pri-background");
    assert_eq!(line.status, "missing-platform-seed");
    assert!(!line.seed_ready);
    assert!(!line.ready);
    assert!(!line.cpu_fallback);
}

#[test]
fn parser_rejects_unknown_commands() {
    assert_eq!(
        parse_command("voice ui"),
        Err(ShellCommandParseError::Unknown)
    );
}
