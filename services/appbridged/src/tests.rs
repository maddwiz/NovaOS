use crate::{
    APPBRIDGED_LAUNCH_SPEC, AppBridgeCommand, AppBridgeManifest, AppBridgeStatus,
    STANDARD_APP_ACTIONS, route_app_action, route_manifest_action,
};
use nova_rt::{
    NovaAgentId, NovaAppActionKind, NovaAppBridgeKind, NovaAppDescriptor, NovaAppId, NovaSceneId,
    NovaServiceId,
};

#[test]
fn launch_action_is_queued_for_matching_app() {
    let app = NovaAppDescriptor {
        id: NovaAppId::new(100),
        name: "notes",
        bridge: NovaAppBridgeKind::Compatibility,
        action_count: 5,
    };
    let command = AppBridgeCommand {
        app: app.id,
        scene: NovaSceneId::ROOT,
        requested_by: NovaAgentId::INIT,
        action: NovaAppActionKind::Launch,
    };

    assert_eq!(
        route_app_action(app, command).status,
        AppBridgeStatus::Queued
    );
}

#[test]
fn standard_manifest_reports_supported_actions() {
    let app = NovaAppDescriptor {
        id: NovaAppId::new(100),
        name: "notes",
        bridge: NovaAppBridgeKind::Compatibility,
        action_count: STANDARD_APP_ACTIONS.len() as u16,
    };
    let manifest = AppBridgeManifest::new(app, STANDARD_APP_ACTIONS);
    let view = manifest.action_view(NovaAppActionKind::RequestAction);

    assert!(manifest.is_valid());
    assert!(manifest.action_count_matches());
    assert!(manifest.supports_action(NovaAppActionKind::Launch));
    assert_eq!(view.app, app.id);
    assert_eq!(view.app_name, "notes");
    assert_eq!(view.bridge, "compatibility");
    assert_eq!(view.action, "request-action");
    assert!(view.supported);
    assert!(view.requires_approval);
}

#[test]
fn manifest_routing_requires_supported_action() {
    let app = NovaAppDescriptor {
        id: NovaAppId::new(200),
        name: "viewer",
        bridge: NovaAppBridgeKind::Native,
        action_count: 1,
    };
    let manifest = AppBridgeManifest::new(app, &[NovaAppActionKind::Open]);
    let unsupported = AppBridgeCommand {
        app: app.id,
        scene: NovaSceneId::ROOT,
        requested_by: NovaAgentId::INIT,
        action: NovaAppActionKind::Close,
    };
    let supported = AppBridgeCommand {
        app: app.id,
        scene: NovaSceneId::ROOT,
        requested_by: NovaAgentId::INIT,
        action: NovaAppActionKind::Open,
    };

    assert!(manifest.is_valid());
    assert_eq!(
        route_manifest_action(manifest, unsupported).status,
        AppBridgeStatus::Unsupported
    );
    assert_eq!(
        route_manifest_action(manifest, supported).status,
        AppBridgeStatus::Queued
    );
    assert_eq!(AppBridgeStatus::Queued.label(), "queued");
}

#[test]
fn manifest_routing_marks_request_action_as_approval_needed() {
    let app = NovaAppDescriptor {
        id: NovaAppId::new(300),
        name: "editor",
        bridge: NovaAppBridgeKind::Remote,
        action_count: STANDARD_APP_ACTIONS.len() as u16,
    };
    let manifest = AppBridgeManifest::new(app, STANDARD_APP_ACTIONS);
    let command = AppBridgeCommand {
        app: app.id,
        scene: NovaSceneId::ROOT,
        requested_by: NovaAgentId::INIT,
        action: NovaAppActionKind::RequestAction,
    };
    let result = route_manifest_action(manifest, command);

    assert_eq!(result.status, AppBridgeStatus::NeedsApproval);
    assert_eq!(result.status.label(), "needs-approval");
}

#[test]
fn launch_spec_identifies_app_bridge_service() {
    assert_eq!(
        APPBRIDGED_LAUNCH_SPEC.descriptor.id,
        NovaServiceId::APPBRIDGED
    );
    assert!(APPBRIDGED_LAUNCH_SPEC.is_valid());
}
