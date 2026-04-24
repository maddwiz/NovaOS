#![no_std]

pub mod launch;
pub mod types;

pub use launch::{SCENED_DESCRIPTOR, SCENED_LAUNCH_SPEC};
pub use types::{
    ROOT_SCENE_BINDINGS, SceneBinding, SceneBindingKind, SceneCheckpoint, SceneManifest,
    SceneRecord, SceneRestorePlan, SceneRestoreStatus, SceneSwitchPlan, SceneSwitchStatus,
    plan_scene_switch, restore_scene, root_scene, root_scene_manifest,
};

#[cfg(test)]
mod tests;
