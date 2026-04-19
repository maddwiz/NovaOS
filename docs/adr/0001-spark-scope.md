# ADR 0001: Spark-First Scope

## Status

Superseded by [ADR 0002: Portable Fabric Scope](/home/nova/NovaOS/docs/adr/0002-portable-fabric.md).

## Context

NovaOS started with one concrete target: NVIDIA DGX Spark / GB10.

The original handoff required a strict platform boundary:
- Arm64 first
- Spark UEFI first
- unified memory first
- integrated GPU first
- no generic-PC broadening

Trying to support additional platforms early would weaken the boot path, the observatory work, and the native GB10 research program.

## Decision

Spark remains the first truth platform, but the repository is now structured for a portable fabric OS with additional NVIDIA platform ports.

The project still defines its system boundary from the UEFI handoff upward and does not treat Linux as the shipped host kernel.

## Consequences

- Loader, kernel, and service design can remain grounded in Spark facts while exposing portable contracts.
- Documentation can record real Spark facts and later workstation/server porting assumptions separately.
- Native GB10 work can still be staged after the boot chain is real.
- Portability work is now part of the architecture, not a deferred afterthought.

## Follow-up

Revisit scope only after:
- the canonical portable-fabric contracts are stable
- the observatory has captured real Spark platform facts
- at least one additional architecture or accelerator personality proves the core contracts survive a port
