#![no_std]

pub mod launch;
pub mod types;

pub use launch::{SHELLD_DESCRIPTOR, SHELLD_LAUNCH_SPEC};
pub use types::{
    ShellAccelDispatchLine, ShellCommand, ShellCommandParseError, ShellMemoryPlacementLine,
    ShellSceneListLine, ShellServiceStatusLine, describe_accel_dispatch, describe_memory_placement,
    describe_scene, describe_service_status, parse_command,
};

#[cfg(test)]
mod tests;
