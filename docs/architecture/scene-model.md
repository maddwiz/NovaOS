# NovaOS Scene Model

Scenes are the durable interaction context above the kernel. A scene groups user intent, agents, apps, and resumable state without turning the kernel into a policy engine.

## Current Code

- `libs/nova_rt::NovaSceneId` identifies a scene.
- `libs/nova_rt::NovaSceneDescriptor` records scene name, mode, owner agent, app count, and agent count.
- `libs/nova_rt::NovaSceneMode` exposes stable labels for shell/operator output.
- `libs/nova_rt::NovaSceneSwitchRequest` carries typed scene-switch requests above the kernel.
- `services/scened` owns the first scene record, binding model, manifest, checkpoint, restore plan, and scene-switch planner.
- `services/shelld` can project scene descriptors into typed scene-list output lines and now forwards `scene` operator commands into the shared scene-switch request path.

## Modes

- `Consumer`: simplified intent-first interaction.
- `Pro`: explicit control and service introspection.
- `Operator`: debug/admin mode for bring-up and validation.

## Next Integration

`scened` now models the save/restore metadata locally: a `SceneManifest` joins a `SceneRecord` with its bindings, `SceneCheckpoint` captures the saved generation plus app/agent/binding counts, and `SceneRestorePlan` reports whether restore is ready, unsaved, or blocked by a binding mismatch. It also accepts typed `NovaSceneSwitchRequest` values and produces a local `SceneSwitchPlan` that distinguishes ready, already-active, missing-target, unknown-target, and restore-blocked transitions.

Real persistence, scene storage, and typed service endpoints remain future integration work.
