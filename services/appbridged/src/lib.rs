#![no_std]

pub mod launch;
pub mod types;

pub use launch::{APPBRIDGED_DESCRIPTOR, APPBRIDGED_LAUNCH_SPEC};
pub use types::{
    AppBridgeActionView, AppBridgeCommand, AppBridgeManifest, AppBridgeResult, AppBridgeStatus,
    STANDARD_APP_ACTIONS, route_app_action, route_manifest_action,
};

#[cfg(test)]
mod tests;
