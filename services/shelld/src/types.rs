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
        "launch policyd" => Ok(ShellCommand::LaunchService(NovaServiceId::POLICYD)),
        "launch agentd" => Ok(ShellCommand::LaunchService(NovaServiceId::AGENTD)),
        "launch intentd" => Ok(ShellCommand::LaunchService(NovaServiceId::INTENTD)),
        "status" => Ok(ShellCommand::Intent(NovaIntentKind::RequestStatus)),
        _ => Err(ShellCommandParseError::Unknown),
    }
}
