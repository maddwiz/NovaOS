#![no_std]

pub mod launch;
pub mod types;

pub use launch::{SHELLD_DESCRIPTOR, SHELLD_LAUNCH_SPEC, SHELLD_PAYLOAD_SPEC};
pub use types::{
    ShellAccelDispatchLine, ShellCommand, ShellCommandParseError, ShellIntentProjectionLine,
    ShellMemoryPlacementLine, ShellSceneListLine, ShellServiceStatusLine, describe_accel_dispatch,
    describe_intent_plan, describe_memory_placement, describe_scene, describe_service_status,
    intent_for_command, parse_command, project_command,
};

#[cfg(test)]
mod tests;
