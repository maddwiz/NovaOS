use nova_rt::{
    NovaAgentId, NovaAppActionKind, NovaAppDescriptor, NovaAppId, NovaSceneId, NovaServiceId,
};

pub const STANDARD_APP_ACTIONS: &[NovaAppActionKind] = &[
    NovaAppActionKind::Launch,
    NovaAppActionKind::Open,
    NovaAppActionKind::Focus,
    NovaAppActionKind::Close,
    NovaAppActionKind::RequestAction,
];

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

impl AppBridgeStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::NeedsApproval => "needs-approval",
            Self::Unsupported => "unsupported",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppBridgeManifest {
    pub descriptor: NovaAppDescriptor,
    pub actions: &'static [NovaAppActionKind],
}

impl AppBridgeManifest {
    pub const fn new(descriptor: NovaAppDescriptor, actions: &'static [NovaAppActionKind]) -> Self {
        Self {
            descriptor,
            actions,
        }
    }

    pub const fn action_count_matches(self) -> bool {
        self.descriptor.action_count as usize == self.actions.len()
    }

    pub const fn is_valid(self) -> bool {
        self.descriptor.id.0 != 0
            && self.descriptor.action_count != 0
            && self.action_count_matches()
    }

    pub const fn supports_action(self, action: NovaAppActionKind) -> bool {
        let mut index = 0usize;
        while index < self.actions.len() {
            if self.actions[index] as u16 == action as u16 {
                return true;
            }
            index += 1;
        }
        false
    }

    pub const fn action_view(self, action: NovaAppActionKind) -> AppBridgeActionView {
        AppBridgeActionView {
            app: self.descriptor.id,
            app_name: self.descriptor.name,
            bridge: self.descriptor.bridge.label(),
            action: action.label(),
            supported: self.supports_action(action),
            requires_approval: matches!(action, NovaAppActionKind::RequestAction),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppBridgeActionView {
    pub app: NovaAppId,
    pub app_name: &'static str,
    pub bridge: &'static str,
    pub action: &'static str,
    pub supported: bool,
    pub requires_approval: bool,
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

pub const fn route_manifest_action(
    manifest: AppBridgeManifest,
    command: AppBridgeCommand,
) -> AppBridgeResult {
    let status =
        if manifest.descriptor.id.0 != command.app.0 || !manifest.supports_action(command.action) {
            AppBridgeStatus::Unsupported
        } else {
            match command.action {
                NovaAppActionKind::RequestAction => AppBridgeStatus::NeedsApproval,
                _ => AppBridgeStatus::Queued,
            }
        };

    AppBridgeResult {
        app: command.app,
        handled_by: NovaServiceId::APPBRIDGED,
        status,
    }
}
