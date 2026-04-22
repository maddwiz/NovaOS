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

    pub const fn checkpoint(self) -> SceneCheckpoint {
        SceneCheckpoint {
            scene: self.descriptor.id,
            saved_generation: self.saved_generation,
            app_count: self.descriptor.app_count,
            agent_count: self.descriptor.agent_count,
            binding_count: self.binding_count,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SceneManifest {
    pub record: SceneRecord,
    pub bindings: &'static [SceneBinding],
}

impl SceneManifest {
    pub const fn new(record: SceneRecord, bindings: &'static [SceneBinding]) -> Self {
        Self { record, bindings }
    }

    pub const fn binding_count_matches(self) -> bool {
        self.record.binding_count as usize == self.bindings.len()
    }

    pub const fn can_restore(self) -> bool {
        self.record.can_restore() && self.binding_count_matches()
    }

    pub const fn checkpoint(self) -> SceneCheckpoint {
        self.record.checkpoint()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SceneCheckpoint {
    pub scene: NovaSceneId,
    pub saved_generation: u64,
    pub app_count: u16,
    pub agent_count: u16,
    pub binding_count: u16,
}

impl SceneCheckpoint {
    pub const fn is_saved(self) -> bool {
        self.saved_generation != 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum SceneRestoreStatus {
    Ready = 1,
    NotSaved = 2,
    BindingMismatch = 3,
}

impl SceneRestoreStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::NotSaved => "not-saved",
            Self::BindingMismatch => "binding-mismatch",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SceneRestorePlan {
    pub scene: NovaSceneId,
    pub status: SceneRestoreStatus,
    pub checkpoint: SceneCheckpoint,
}

impl SceneRestorePlan {
    pub const fn ready(self) -> bool {
        matches!(self.status, SceneRestoreStatus::Ready)
    }
}

pub const fn restore_scene(manifest: SceneManifest) -> SceneRestorePlan {
    let status = if !manifest.record.can_restore() {
        SceneRestoreStatus::NotSaved
    } else if !manifest.binding_count_matches() {
        SceneRestoreStatus::BindingMismatch
    } else {
        SceneRestoreStatus::Ready
    };

    SceneRestorePlan {
        scene: manifest.record.descriptor.id,
        status,
        checkpoint: manifest.checkpoint(),
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

pub const ROOT_SCENE_BINDINGS: &[SceneBinding] =
    &[SceneBinding::agent(NovaSceneId::ROOT, NovaAgentId::INIT)];

pub const fn root_scene_manifest() -> SceneManifest {
    SceneManifest::new(root_scene(), ROOT_SCENE_BINDINGS)
}
