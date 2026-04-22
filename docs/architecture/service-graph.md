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

`apps/initd` also publishes a static boot status page for that first runtime spine. Required services are reported as started/running, while optional `shelld` is reported as deferred until an operator shell boundary is needed. The host-side `initd-runtime` binary now prints the joined status/policy/kernel-binding report for operator inspection.

## Launch Manifest V0

`libs/nova_rt::service` now defines a local launch manifest model:

- `NovaServiceLaunchSpec` binds a service descriptor to bootstrap capability, endpoint-slot, and shared-memory requirements.
- `NovaServiceKernelBinding` names the future task, control endpoint, shared-memory region, binding state, and health generation for a service.
- `NovaServiceKernelLaunchPlan` ties the descriptor, launch request, and future kernel binding together.

Each service crate exports its own descriptor and launch spec. `apps/initd` assembles those service-owned specs into the first runtime manifest and publishes a deterministic kernel-binding plan for required services. Optional `shelld` remains model-only until an operator shell boundary is requested.

`apps/initd` also asks `services/policyd` for the launch decision attached to each service report. The current static policy matrix allows launch of the known runtime-spine services and denies unknown service targets, and `policyd` can now emit a typed audit record for each evaluated request. The first chain remains boot-green while making policyd part of the runtime seam instead of a detached placeholder.

This step does not allocate syscall numbers, does not modify `kernel/**`, and does not change boot handoff behavior.

The shared IDs, descriptors, service statuses, launch specs, kernel bindings, launch requests, launch results, policy decisions, intent envelopes, scene IDs, app IDs, and agent IDs live in `libs/nova_rt::service`.

`services/shelld` now parses launch commands for the full first runtime spine and exposes typed service-status and scene-list output lines over generic `nova_rt` descriptors. `services/scened` now owns typed scene manifests, checkpoints, restore plans, and root scene bindings. `services/appbridged` now owns manifest-backed app action support and routes supported, unsupported, and approval-needed app actions. `services/intentd` has locked route coverage for status, scene switching, app opening, launch requests, explicit target overrides, and scene-scoped policy request projection into `policyd`. `services/agentd` now owns agent lifecycle labels, runtime records, quota snapshots/decisions for tool grants, service delegation, and memory pages, plus scene participation checks. This gives the operator shell, scene manager, app bridge, policy service, intent router, and agent manager a shared service vocabulary without introducing kernel dependencies.

This is still a local model, not a true process launch graph. Real kernel task creation, endpoint wiring, shared-memory grants, and kernel-backed service health publication remain future integration work.
