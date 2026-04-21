use nova_rt::{
    NovaAgentId, NovaAppActionKind, NovaAppDescriptor, NovaAppId, NovaSceneId, NovaServiceId,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppBridgeCommand {
    pub app: NovaAppId,
    pub scene: NovaSceneId,
    pub requested_by: NovaAgentId,
    pub action: NovaAppActionKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum AppBridgeStatus {
    Queued = 1,
    NeedsApproval = 2,
    Unsupported = 3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppBridgeResult {
    pub app: NovaAppId,
    pub handled_by: NovaServiceId,
    pub status: AppBridgeStatus,
}

pub const fn route_app_action(
    descriptor: NovaAppDescriptor,
    command: AppBridgeCommand,
) -> AppBridgeResult {
    let status = if descriptor.id.0 != command.app.0 {
        AppBridgeStatus::Unsupported
    } else {
        match command.action {
            NovaAppActionKind::Launch
            | NovaAppActionKind::Open
            | NovaAppActionKind::Focus
            | NovaAppActionKind::Close => AppBridgeStatus::Queued,
            NovaAppActionKind::RequestAction => AppBridgeStatus::NeedsApproval,
        }
    };

    AppBridgeResult {
        app: command.app,
        handled_by: NovaServiceId::APPBRIDGED,
        status,
    }
}
