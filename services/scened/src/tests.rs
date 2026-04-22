use crate::{SCENED_LAUNCH_SPEC, SceneBinding, SceneBindingKind, root_scene};
use nova_rt::{NovaAgentId, NovaSceneId, NovaServiceId};

#[test]
fn root_scene_is_restore_ready() {
    let scene = root_scene();
    assert_eq!(scene.descriptor.id, NovaSceneId::ROOT);
    assert!(scene.can_restore());
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
