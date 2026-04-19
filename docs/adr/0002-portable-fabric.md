# ADR 0002: Portable Fabric Scope

## Status

Accepted.

## Context

NovaOS started with Spark as the first truth platform. That remains true, but the project must not lock itself into Spark-only assumptions if it is meant to grow into a portable NVIDIA fabric OS.

The initial scope still matters:
- Spark is the first real hardware target
- Spark UEFI remains the boot oracle
- Spark / GB10 remains the first accelerator truth source

The architecture must also support later x86_64 workstation and server ports without rewriting the core model.

## Decision

NovaOS will keep Spark first, but the repository will be structured around a portable core plus platform and accelerator personalities.

The project will:
- own the UEFI handoff upward
- keep kernel policy minimal
- keep accelerator policy in userspace
- isolate platform-specific facts behind platform modules and backend personalities
- treat Linux-based vendor stacks as research oracles only

## Consequences

- Spark implementation work stays grounded in real hardware facts.
- Future workstation and server ports can reuse the same service, capability, and fabric contracts.
- Deep driver work must be divided by backend personality instead of encoded in generic kernel or service code.

## Follow-up

The repo should maintain:
- a canonical roadmap checklist
- a canonical live status file
- a portability/fabric architecture note
- an explicit rule that Spark-specific facts do not leak into Ring A
