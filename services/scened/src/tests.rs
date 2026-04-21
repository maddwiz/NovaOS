use crate::{SceneBinding, SceneBindingKind, root_scene};
use nova_rt::{NovaAgentId, NovaSceneId};

#[test]
fn root_scene_is_restore_ready() {
    let scene = root_scene();
    assert_eq!(scene.descriptor.id, NovaSceneId::ROOT);
    assert!(scene.can_restore());
}

#[test]
fn scene_can_bind_agents() {
    let binding = SceneBinding::agent(NovaSceneId::ROOT, NovaAgentId::new(9));
    assert_eq!(binding.kind, SceneBindingKind::Agent);
    assert_eq!(binding.object_id, 9);
}
