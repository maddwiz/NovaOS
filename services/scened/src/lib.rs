#![no_std]

pub mod types;

pub use types::{SceneBinding, SceneBindingKind, SceneRecord, root_scene};

#[cfg(test)]
mod tests;
