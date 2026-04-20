# BootInfo v2 Draft

BootInfo v2 is the portable-fabric successor to the current live BootInfo v1 handoff.

Spark remains the first truth platform, but the boot contract can no longer be shaped only around one Arm64 UMA machine. The v2 draft introduces the minimum fields needed for later x86_64, discrete PCIe, and partitioned fabric lanes without breaking the current Spark boot path.

## New contract goals

- identify CPU architecture and platform class explicitly
- identify memory topology class explicitly
- preserve optional firmware-table families beyond ACPI, DTB, and SMBIOS
- seed later storage, network, and accelerator discovery from the loader
- carry both kernel and loader image hashes
- support an optional observatory hash so report artifacts can be correlated with boot handoff state
- carry an optional bootstrap-task payload descriptor so later stages can keep the embedded `initd` image facts without reparsing the capsule body
- carry an optional bootstrap user-window descriptor so the EL0 path has an explicit page-aligned address-space contract instead of reusing firmware config-table fields

## Current status

- `abi/boot/nova_bootinfo_v2.h` is the active draft.
- the loader now carries an internal/transitional BootInfo v2 sidecar, stage1 validates it, and the raw live kernel entry now resolves and records a compact validated v2 summary from that sidecar while current bringup still derives its primary state from BootInfo v1.
- that sidecar now also snapshots the embedded bootstrap-task payload image pointer, size, load window, and entry point when `init.capsule` carries a valid wrapped `BOOTSTRAP_TASK_V1` body.
- the sidecar has an empty-by-default bootstrap user-window descriptor in the ABI, and `novaaa64` now populates it when a valid embedded bootstrap payload exists. This is a portable virtual-address policy for EL0 image/context/stack placement, not a hardware-specific physical allocation or MMU-enable step.
- no code should deepen the v1 contract without checking whether the field really belongs in the v2 draft instead.

## Stop-the-world rule

Before deepening platform-specific boot work again, the repo must keep these v2-era abstractions visible:

- platform class enum
- CPU architecture enum
- memory-topology class enum
- accelerator seed abstraction
- portable fabric/backend interfaces in userspace services

That keeps later Spark, RTX, and Hopper lanes from turning into a rewrite.
