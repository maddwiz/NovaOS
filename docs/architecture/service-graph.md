# NovaOS Service Graph

NovaOS keeps the kernel mechanism-only. Policy, agents, intent routing, scenes, app bridging, memory policy, and accelerator policy live above the kernel as services.

This service graph is an additive runtime layer inside the existing M0-M17 roadmap. It does not replace the Spark/QEMU boot-continuity and portability milestones.

## Layers

- Kernel mechanisms: task, capability, endpoint, shared-memory, event/logging, and scheduling primitives.
- Core services: `initd`, `policyd`, `agentd`, `memd`, and `acceld`.
- Interaction services: `intentd`, `scened`, `appbridged`, and `shelld`.
- Future presentation: consumer shell, pro shell, scene UI, and app fallback UI.

## First Runtime Spine

`apps/initd` now carries a typed launch table for:

- `policyd`
- `agentd`
- `memd`
- `acceld`
- `intentd`
- `scened`
- `appbridged`
- optional `shelld`

`apps/initd` also publishes a static boot status page for that first runtime spine. Required services are reported as started/running, while optional `shelld` is reported as deferred until an operator shell boundary is needed.

The shared IDs, descriptors, service statuses, launch requests, launch results, policy decisions, intent envelopes, scene IDs, app IDs, and agent IDs live in `libs/nova_rt::service`.

This is still a local model, not a true process launch graph. Real kernel task creation, endpoint wiring, shared-memory grants, and kernel-backed service health publication remain future integration work.
