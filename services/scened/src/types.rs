use nova_rt::{NovaAgentId, NovaAppId, NovaSceneDescriptor, NovaSceneId, NovaSceneMode};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum SceneBindingKind {
    Agent = 1,
    App = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SceneBinding {
    pub scene: NovaSceneId,
    pub kind: SceneBindingKind,
    pub object_id: u64,
}

impl SceneBinding {
    pub const fn agent(scene: NovaSceneId, agent: NovaAgentId) -> Self {
        Self {
            scene,
            kind: SceneBindingKind::Agent,
            object_id: agent.0,
        }
    }

    pub const fn app(scene: NovaSceneId, app: NovaAppId) -> Self {
        Self {
            scene,
            kind: SceneBindingKind::App,
            object_id: app.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SceneRecord {
    pub descriptor: NovaSceneDescriptor,
    pub saved_generation: u64,
    pub binding_count: u16,
}

impl SceneRecord {
    pub const fn can_restore(self) -> bool {
        self.saved_generation != 0
    }
}

pub const fn root_scene() -> SceneRecord {
    SceneRecord {
        descriptor: NovaSceneDescriptor {
            id: NovaSceneId::ROOT,
            name: "root",
            mode: NovaSceneMode::Pro,
            owner_agent: NovaAgentId::INIT,
            app_count: 0,
            agent_count: 1,
        },
        saved_generation: 1,
        binding_count: 1,
    }
}
