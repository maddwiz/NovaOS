use nova_rt::{NovaIntentKind, NovaSceneDescriptor, NovaSceneId, NovaServiceId, NovaServiceStatus};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShellCommand {
    Services,
    Scenes,
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

pub fn parse_command(input: &str) -> Result<ShellCommand, ShellCommandParseError> {
    match input.trim() {
        "" => Err(ShellCommandParseError::Empty),
        "services" | "svc" => Ok(ShellCommand::Services),
        "scenes" | "scene ls" => Ok(ShellCommand::Scenes),
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
