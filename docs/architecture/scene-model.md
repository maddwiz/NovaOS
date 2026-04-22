# NovaOS Scene Model

Scenes are the durable interaction context above the kernel. A scene groups user intent, agents, apps, and resumable state without turning the kernel into a policy engine.

## Current Code

- `libs/nova_rt::NovaSceneId` identifies a scene.
- `libs/nova_rt::NovaSceneDescriptor` records scene name, mode, owner agent, app count, and agent count.
- `libs/nova_rt::NovaSceneMode` exposes stable labels for shell/operator output.
- `services/scened` owns the first scene record, binding model, manifest, checkpoint, and restore plan.
- `services/shelld` can project scene descriptors into typed scene-list output lines without depending on `scened`.

## Modes

- `Consumer`: simplified intent-first interaction.
- `Pro`: explicit control and service introspection.
- `Operator`: debug/admin mode for bring-up and validation.

## Next Integration

`scened` now models the save/restore metadata locally: a `SceneManifest` joins a `SceneRecord` with its bindings, `SceneCheckpoint` captures the saved generation plus app/agent/binding counts, and `SceneRestorePlan` reports whether restore is ready, unsaved, or blocked by a binding mismatch.

Real persistence, scene storage, and typed service endpoints remain future integration work.
