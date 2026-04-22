#![no_std]

pub mod launch;
pub mod types;

pub use launch::{SHELLD_DESCRIPTOR, SHELLD_LAUNCH_SPEC};
pub use types::{
    ShellCommand, ShellCommandParseError, ShellSceneListLine, ShellServiceStatusLine,
    describe_scene, describe_service_status, parse_command,
};

#[cfg(test)]
mod tests;
