use crate::{
    ROOT_SCENE_BINDINGS, SCENED_LAUNCH_SPEC, SCENED_PAYLOAD_SPEC, SceneBinding, SceneBindingKind,
    SceneManifest, SceneRestoreStatus, SceneSwitchStatus, plan_scene_switch, restore_scene,
    root_scene, root_scene_manifest,
};
use nova_rt::{
    NovaAgentId, NovaPayloadEntryAbi, NovaPayloadKind, NovaSceneId, NovaSceneSwitchRequest,
    NovaServiceId,
};

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
    assert_eq!(SCENED_LAUNCH_SPEC.artifact, Some(SCENED_PAYLOAD_SPEC));
    assert_eq!(SCENED_PAYLOAD_SPEC.image_stem, "scened-payload");
    assert_eq!(SCENED_PAYLOAD_SPEC.payload_kind, NovaPayloadKind::Service);
    assert_eq!(
        SCENED_PAYLOAD_SPEC.entry_abi,
        NovaPayloadEntryAbi::BootstrapTaskV1
    );
    assert!(!SCENED_PAYLOAD_SPEC.embedded_in_init_capsule);
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

#[test]
fn scene_switch_plan_is_ready_for_known_restorable_target() {
    let manifest = root_scene_manifest();
    let request =
        NovaSceneSwitchRequest::new(NovaAgentId::INIT, NovaSceneId::new(9), NovaSceneId::ROOT);
    let plan = plan_scene_switch(manifest, request);

    assert!(plan.ready());
    assert_eq!(plan.status, SceneSwitchStatus::Ready);
    assert_eq!(plan.status.label(), "ready");
    assert_eq!(plan.target_scene, NovaSceneId::ROOT);
}

#[test]
fn scene_switch_plan_detects_already_active_target() {
    let manifest = root_scene_manifest();
    let request =
        NovaSceneSwitchRequest::new(NovaAgentId::INIT, NovaSceneId::ROOT, NovaSceneId::ROOT);
    let plan = plan_scene_switch(manifest, request);

    assert!(!plan.ready());
    assert_eq!(plan.status, SceneSwitchStatus::AlreadyActive);
    assert_eq!(plan.status.label(), "already-active");
}

#[test]
fn scene_switch_plan_rejects_unresolved_target_scene() {
    let manifest = root_scene_manifest();
    let request = NovaSceneSwitchRequest::unresolved(NovaAgentId::INIT, NovaSceneId::ROOT);
    let plan = plan_scene_switch(manifest, request);

    assert!(!plan.ready());
    assert_eq!(plan.status, SceneSwitchStatus::MissingTargetScene);
    assert_eq!(plan.status.label(), "missing-target-scene");
}

#[test]
fn scene_switch_plan_rejects_restore_blocked_target() {
    let mut scene = root_scene();
    scene.saved_generation = 0;
    let manifest = SceneManifest::new(scene, ROOT_SCENE_BINDINGS);
    let request =
        NovaSceneSwitchRequest::new(NovaAgentId::INIT, NovaSceneId::new(9), NovaSceneId::ROOT);
    let plan = plan_scene_switch(manifest, request);

    assert!(!plan.ready());
    assert_eq!(plan.status, SceneSwitchStatus::RestoreBlocked);
    assert_eq!(plan.status.label(), "restore-blocked");
}
