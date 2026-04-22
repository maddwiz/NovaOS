use crate::{APPBRIDGED_LAUNCH_SPEC, AppBridgeCommand, AppBridgeStatus, route_app_action};
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
fn launch_spec_identifies_app_bridge_service() {
    assert_eq!(
        APPBRIDGED_LAUNCH_SPEC.descriptor.id,
        NovaServiceId::APPBRIDGED
    );
    assert!(APPBRIDGED_LAUNCH_SPEC.is_valid());
}
