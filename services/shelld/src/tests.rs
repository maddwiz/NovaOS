use crate::{
    SHELLD_LAUNCH_SPEC, ShellCommand, ShellCommandParseError, describe_scene,
    describe_service_status, parse_command,
};
use nova_rt::{
    NovaAgentId, NovaSceneDescriptor, NovaSceneId, NovaSceneMode, NovaServiceDescriptor,
    NovaServiceId, NovaServiceKind, NovaServiceLaunchResult, NovaServiceLaunchStatus,
    NovaServiceState, NovaServiceStatus,
};

#[test]
fn parser_lists_services() {
    assert_eq!(parse_command("services"), Ok(ShellCommand::Services));
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
fn parser_rejects_unknown_commands() {
    assert_eq!(
        parse_command("voice ui"),
        Err(ShellCommandParseError::Unknown)
    );
}
