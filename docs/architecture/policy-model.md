# NovaOS Policy Model

`policyd` is the central service authority for grants, approvals, scopes, and audit. The kernel enforces mechanisms; policy decisions stay in services.

## Current Code

- `libs/nova_rt::NovaPolicyDecision`: `Allow`, `Deny`, or `Ask`.
- `libs/nova_rt::NovaPolicyAction`: launch service, stop service, route intent, access memory, app action, and delegate to agent.
- `libs/nova_rt::NovaPolicyScope`: system, service, scene, agent, or app.
- `services/policyd` owns the first hardcoded decision matrix.
- `apps/initd` evaluates each service launch in its runtime report through the `policyd` matrix before reporting the launch as policy-allowed.
- `services/intentd` projects each intent into a scene-scoped `RouteIntent` policy request before routing.

## Initial Defaults

- Service launch is allowed only for the known runtime-spine services in the static matrix.
- Intent routing, agent delegation, and app action require approval.
- Memory visibility defaults to deny.
- System-scoped rules act as global defaults for the same action, so scene-scoped route requests inherit the ask-before-action rule.

The first matrix is intentionally conservative and static. Dynamic policy loading, audit persistence, and user approval UX are future service-layer work.
