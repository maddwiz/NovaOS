# NovaOS App Bridge

`appbridged` is where NovaOS preserves access to existing app ecosystems without making the desktop metaphor the primary UI model.

## Current Code

- `libs/nova_rt::NovaAppId` identifies an app.
- `libs/nova_rt::NovaAppDescriptor` records bridge kind and action count.
- `libs/nova_rt::NovaAppBridgeKind` and `libs/nova_rt::NovaAppActionKind` expose stable labels for operator/service output.
- `libs/nova_rt::NovaAppActionKind` covers `launch`, `open`, `focus`, `close`, and `request-action`.
- `libs/nova_rt::NovaAppActionRequest` carries typed app-action requests above the kernel.
- `services/appbridged` owns `AppBridgeManifest`, which binds an app descriptor to its supported action list.
- `services/appbridged` routes manifest-backed app actions into queued, approval-needed, or unsupported results, including shared `NovaAppActionRequest` inputs projected by `intentd`.

## Boundaries

The bridge is not a kernel driver and does not imply Linux host-driver dependency in the shipping runtime path. It is a service-level compatibility and action-exposure seam.

Future store integration, compatibility runtimes, and richer app action schemas should land here, not in the kernel. The current manifest model is still local/static; it does not launch real applications yet.
