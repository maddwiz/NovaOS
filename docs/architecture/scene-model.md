# NovaOS Scene Model

Scenes are the durable interaction context above the kernel. A scene groups user intent, agents, apps, and resumable state without turning the kernel into a policy engine.

## Current Code

- `libs/nova_rt::NovaSceneId` identifies a scene.
- `libs/nova_rt::NovaSceneDescriptor` records scene name, mode, owner agent, app count, and agent count.
- `libs/nova_rt::NovaSceneMode` exposes stable labels for shell/operator output.
- `services/scened` owns the first scene record and binding model.
- `services/shelld` can project scene descriptors into typed scene-list output lines without depending on `scened`.

## Modes

- `Consumer`: simplified intent-first interaction.
- `Pro`: explicit control and service introspection.
- `Operator`: debug/admin mode for bring-up and validation.

## Next Integration

`scened` should eventually persist scene metadata, bind agents/apps to scene IDs, and expose save/restore through typed service endpoints.
