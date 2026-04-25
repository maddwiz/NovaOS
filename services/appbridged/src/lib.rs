#![no_std]

pub mod launch;
pub mod types;

pub use launch::{APPBRIDGED_DESCRIPTOR, APPBRIDGED_LAUNCH_SPEC, APPBRIDGED_PAYLOAD_SPEC};
pub use types::{
    AppBridgeActionView, AppBridgeCommand, AppBridgeManifest, AppBridgeResult, AppBridgeStatus,
    STANDARD_APP_ACTIONS, app_request_command, route_app_action, route_app_request,
    route_manifest_action, route_manifest_request,
};

#[cfg(test)]
mod tests;
