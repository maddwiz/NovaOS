# NovaOS Portable Fabric

NovaOS is a from-scratch operating system for NVIDIA compute platforms. Spark remains the first truth platform, but the core architecture must survive later x86_64 workstation and server ports.

## Core Rules

- Keep the kernel mechanism-only.
- Keep accelerator policy in userspace.
- Keep the portable core free of Spark-specific and GB10-specific assumptions.
- Treat the repository as a portable fabric OS, not a single-machine experiment.
- Keep the shipping runtime independent from Linux host kernels and Linux driver bridges.

## Stability Rings

### Ring A: invariant core

- service model
- capability model
- package and update model
- IPC and shared-memory model
- audit model
- agent authority model
- model lifecycle model
- generic memory-policy APIs
- fabric contract
- telemetry schemas
- scheduler concepts

### Ring B: platform ports

- Arm64 and x86_64 kernel arch code
- UEFI loader variants
- page tables and interrupt handling
- bus discovery
- IOMMU and DMA details
- platform table parsing

### Ring C: accelerator personalities

- Spark / GB10 integrated personality
- RTX PCIe personality
- Hopper / H100 fabric personality
- future NVIDIA families

## Handoff Doctrine

- Spark remains the first hardware oracle.
- Portability hooks must land before deep kernel ossification.
- Any GPU assumption in Ring A is a bug.
- Platform-specific facts belong behind `platform/` and `drivers/`.

## Resume Rule

Use `docs/roadmap/live-status.md` and `docs/roadmap/master-roadmap-checklist.md` as the canonical resume source before making changes.
