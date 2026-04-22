use crate::{
    ROOT_SCENE_BINDINGS, SCENED_LAUNCH_SPEC, SceneBinding, SceneBindingKind, SceneManifest,
    SceneRestoreStatus, restore_scene, root_scene, root_scene_manifest,
};
use nova_rt::{NovaAgentId, NovaSceneId, NovaServiceId};

#[test]
fn root_scene_is_restore_ready() {
    let scene = root_scene();
    assert_eq!(scene.descriptor.id, NovaSceneId::ROOT);
    assert!(scene.can_restore());
    assert_eq!(scene.checkpoint().saved_generation, 1);
    assert_eq!(ROOT_SCENE_BINDINGS.len(), 1);
}

#[test]
fn launch_spec_identifies_scene_service() {
    assert_eq!(SCENED_LAUNCH_SPEC.descriptor.id, NovaServiceId::SCENED);
    assert!(SCENED_LAUNCH_SPEC.is_valid());
}

#[test]
fn scene_can_bind_agents() {
    let binding = SceneBinding::agent(NovaSceneId::ROOT, NovaAgentId::new(9));
    assert_eq!(binding.kind, SceneBindingKind::Agent);
    assert_eq!(binding.object_id, 9);
}

#[test]
fn root_manifest_restores_when_checkpoint_and_bindings_match() {
    let manifest = root_scene_manifest();
    let plan = restore_scene(manifest);

    assert!(manifest.binding_count_matches());
    assert!(manifest.can_restore());
    assert!(plan.ready());
    assert_eq!(plan.status, SceneRestoreStatus::Ready);
    assert_eq!(plan.status.label(), "ready");
    assert_eq!(plan.checkpoint.scene, NovaSceneId::ROOT);
    assert_eq!(plan.checkpoint.agent_count, 1);
    assert!(plan.checkpoint.is_saved());
}

#[test]
fn restore_plan_rejects_unsaved_scene_metadata() {
    let mut scene = root_scene();
    scene.saved_generation = 0;
    let manifest = SceneManifest::new(scene, ROOT_SCENE_BINDINGS);
    let plan = restore_scene(manifest);

    assert!(!manifest.can_restore());
    assert!(!plan.ready());
    assert_eq!(plan.status, SceneRestoreStatus::NotSaved);
    assert_eq!(plan.status.label(), "not-saved");
}

#[test]
fn restore_plan_rejects_binding_count_mismatch() {
    let mut scene = root_scene();
    scene.binding_count = 2;
    let manifest = SceneManifest::new(scene, ROOT_SCENE_BINDINGS);
    let plan = restore_scene(manifest);

    assert!(!manifest.binding_count_matches());
    assert_eq!(plan.status, SceneRestoreStatus::BindingMismatch);
    assert_eq!(plan.status.label(), "binding-mismatch");
}
