# NovaOS App Bridge

`appbridged` is where NovaOS preserves access to existing app ecosystems without making the desktop metaphor the primary UI model.

## Current Code

- `libs/nova_rt::NovaAppId` identifies an app.
- `libs/nova_rt::NovaAppDescriptor` records bridge kind and action count.
- `libs/nova_rt::NovaAppActionKind` covers `launch`, `open`, `focus`, `close`, and `request-action`.
- `services/appbridged` routes app actions into queued, approval-needed, or unsupported results.

## Boundaries

The bridge is not a kernel driver and does not imply Linux host-driver dependency in the shipping runtime path. It is a service-level compatibility and action-exposure seam.

Future store integration, compatibility runtimes, and richer app action schemas should land here, not in the kernel.
