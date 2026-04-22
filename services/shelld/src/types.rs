use nova_rt::{NovaIntentKind, NovaSceneId, NovaServiceId};

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
