use crate::{SHELLD_LAUNCH_SPEC, ShellCommand, ShellCommandParseError, parse_command};
use nova_rt::NovaServiceId;

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
}

#[test]
fn parser_rejects_unknown_commands() {
    assert_eq!(
        parse_command("voice ui"),
        Err(ShellCommandParseError::Unknown)
    );
}
