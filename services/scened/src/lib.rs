#![no_std]

pub mod launch;
pub mod types;

pub use launch::{SCENED_DESCRIPTOR, SCENED_LAUNCH_SPEC};
pub use types::{SceneBinding, SceneBindingKind, SceneRecord, root_scene};

#[cfg(test)]
mod tests;
