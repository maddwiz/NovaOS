# NovaOS Master Roadmap Checklist

This is the durable handoff document for NovaOS. Another Codex session should read this file, then the locally generated `artifacts/reports/latest-status.txt` and `artifacts/reports/latest-loop-status.txt` when present, before changing code.

## Scope

- NovaOS is now a portable NVIDIA fabric OS with Spark as the first truth platform.
- Spark remains the first shipping lane and the first serious hardware target.
- The shipped runtime must remain NovaOS from the UEFI handoff upward, not Linux.
- Generic contracts must survive Spark, later RTX workstation lanes, and later Hopper-class lab lanes.

## Operator Rule

- Treat this checklist and `docs/roadmap/live-status.md` as the canonical resume source.
- Keep them current whenever a meaningful architecture, ABI, boot-contract, or milestone change lands.
- Preserve Spark bootability while portability refactors happen.

## Stop-The-World Portability Gate

Before deeper platform-specific kernel work continues, keep these items true:

- [x] a portable fabric contract exists in code or docs
- [x] a BootInfo v2 draft exists
- [x] a platform class enum exists
- [x] an accelerator seed abstraction exists
- [x] `acceld` has backend traits even if only placeholders exist
- [x] `memd` has profile interfaces even if only placeholders exist
- [x] `kernel/arch/x86_64/` exists
- [x] `drivers/bus/pci/` exists
- [x] the live Spark lane now plans around BootInfo v2 seeds internally while the raw kernel entry carries an optional validated BootInfo v2 sidecar and current bringup still uses BootInfo v1 as the primary contract
- [x] the observatory path is upgraded to emit facts that map cleanly into BootInfo v2 seeds

## Current State

- The repository is NovaOS.
- The 24/7 local validation loop remains the default continuity mechanism.
- QEMU Arm64 UEFI stage0 -> stage1 -> kernel is green.
- Spark is still the only lane with a real boot path.
- `BootInfo v1` is still the primary live handoff used by the loader and current kernel bringup.
- `BootInfo v2` now exists as both a C ABI draft and a Rust runtime contract for the portability refactor, and the loader now keeps an internal/transitional v2 sidecar with display/storage/network path facts plus one integrated/UMA accelerator seed that stage1 carries into the raw kernel entry under a dedicated payload ABI while current bringup still uses `BootInfo v1` as the primary contract.
- `Nova Payload v1` carries an explicit flat-binary load window and entry ABI.
- Stage0 publishes persistent digest and verification metadata.
- `libs/nova_fabric` now carries generic platform, transport, topology, pool, queue, and accelerator-seed contracts.
- `services/acceld` and `services/memd` now expose backend/profile interfaces with placeholder CPU, GB10, RTX, Hopper, UMA, discrete, NVLink, and MIG lanes.
- `services/policyd`, `services/agentd`, `services/intentd`, `services/scened`, `services/appbridged`, and `services/shelld` now exist as additive runtime-spine service crates backed by shared `nova_rt` service, policy, agent, intent, scene, and app bridge contracts.
- `kernel/arch/x86_64` and `drivers/bus/pci` now exist as placeholder lanes.
- `kernel/arch/arm64/src/lib.rs` is being reduced into focused modules; boot-contract parsing, EL transfer helpers, and diagnostic probes now live outside the root orchestrator.
- `abi/syscall/nova_syscall_v1.h` and `nova_rt` now define the first typed syscall ABI draft, and `kernel/arch/arm64` now carries the Arm64 `svc` scaffold dispatcher, gated by the typed bootstrap task capability mask and bounded by bootstrap endpoint/shared-memory quotas, with the first reserved bootstrap endpoint and shared-memory lanes now returning real kernel-owned results in the QEMU probe.
- `init.capsule` now builds as a typed `InitCapsule v1` artifact with an embedded bootstrap service payload body, stage0/stage1 validate it before handoff continues, the loader snapshots an explicit bootstrap-payload descriptor plus populated portable bootstrap user-window and loader-reserved bootstrap frame-arena descriptors into `BootInfo v2` when the embedded payload is valid, and the raw Arm64 kernel records a compact bootstrap task summary plus the first in-place bootstrap payload transfer with a typed bootstrap context, explicit unisolated current bootstrap boundary plan, and isolated `drop-to-el0`/`el0-svc` target boundary plan in QEMU while hardening the live handoff around the EL1 bootstrap stack banks. The kernel payload wrapper now also pads the kernel body to an alignment-safe `load_offset`, so stage1's existing in-place `load_base` contract no longer leaves the live vector table misaligned. That same context still carries a transitional same-EL bootstrap kernel-call gate, so the current QEMU lane proves a real `initd -> kernel -> initd` round-trip while still landing with `current_el=1`, and the default smoke lane now also proves a payload-originated same-EL `svc` return. The AArch64 vector table now also separates the lower-EL AArch64 sync slot and the default smoke lane runs a kernel-owned lower-EL `svc` dry-run so future EL0 `svc` returns can use the normal ELR-advance path. A feature-gated raw no-MMU EL0 diagnostic now also proves EL1-to-EL0 `eret`, lower-EL `svc` dispatch from `initd`, and return to EL0 spin, while the kernel now logs ready typed EL0 user-window, user-mapping, backing-frame arena, page-table descriptor plans, populated payload/context/stack backing frames, constructed transitional AArch64 translation-table pages, and prepared TTBR/TCR/MAIR values for the proposed user image window. Kernel-owned and pre-transfer live `svc` return remain proven on diagnostic lanes, and payload-originated live `brk` return remains proven.
- `kernel/arch/arm64` now also carries feature-gated `bootstrap_kernel_svc_probe`, `bootstrap_pretransfer_svc_probe`, and `bootstrap_el0_probe` lanes, `apps/initd` carries feature-gated `bootstrap_svc_probe`, `bootstrap_trap_probe`, and `bootstrap_el0_probe` lanes, and the dedicated QEMU diagnostic runners capture the current same-EL live exception status plus the raw EL0 probe without weakening the default QEMU smoke baseline.
- `spark-observe` now emits a persisted structured report with loaded-image and display device paths, storage/network seed paths, and a transitional Spark accelerator-seed draft in addition to framebuffer and table state.
- `novaaa64` now emits a persisted structured loader handoff report with BootInfo v2 readiness and payload-presence facts before the live handoff exits boot services.
- `scripts/compare-spark-observe-reports.sh` now compares a real Spark observatory report against the persisted QEMU baseline and writes a stable comparison artifact.
- `scripts/prepare-spark-hardware-bundle.sh` now stages observatory and loader bundles under `artifacts/hardware/`, `scripts/install-spark-hardware-bundle.sh` can install them onto a mounted ESP and optionally schedule `BootNext`, `scripts/collect-spark-observe-report.sh` ingests a returned observatory report back into `artifacts/reports/`, `scripts/collect-novaaa64-loader-report.sh` ingests and validates a returned loader handoff report, `scripts/check-spark-stage-chain-proof.sh` now validates returned stage0 -> stage1 -> kernel evidence against the current QEMU markers, `scripts/finalize-spark-hardware-proof.sh` writes the combined hardware-proof summary, and `scripts/complete-spark-hardware-proof.sh` now wraps the full post-boot return path in one command.
- Real Spark hardware acceptance is still pending for the observatory, loader, and kernel milestones.

## Resume Order

1. Read `artifacts/reports/latest-status.txt` when present, or run `make report` first in a fresh clone.
2. Read `artifacts/reports/latest-loop-status.txt` when present.
3. Read `artifacts/reports/latest-report.md` when present.
4. Read `docs/roadmap/live-status.md`.
5. Continue from the first unchecked item below.

## Immediate Discipline

1. Keep Spark booting while the portability scaffolding lands.
2. Do not add GB10- or Spark-specific facts to generic contracts in `libs/`, `kernel/core`, or generic services.
3. Do not add PCIe workstation assumptions to generic contracts either.
4. Update the checklist and live status whenever the architecture direction changes.

## Master Checklist

### M0 Repository Refactor For Portability

- Acceptance: the repo tree supports a portable-fabric direction while the Spark lane still builds and validates.
- [x] `libs/nova_fabric/` exists.
- [x] `services/acceld/` exists with backend traits and placeholder backends.
- [x] `services/memd/` exists with profile traits and placeholder profiles.
- [x] first additive AI-native runtime-spine service crates exist above the kernel.
- [x] Arm64 root module split has started with boot-contract, EL, and diagnostic modules.
- [x] `kernel/arch/x86_64/` exists.
- [x] `drivers/bus/pci/` exists.
- [x] `drivers/gpu/nvidia/common/`, `gb10/`, `rtx_pcie/`, and `hopper_fabric/` exist.
- [x] top-level `platform/` lanes exist for Spark, workstation, and Hopper lab placeholders.
- [x] BootInfo v2 draft exists in `abi/boot/` and `docs/boot/`.
- [x] portable-fabric architecture notes exist.
- [x] the live Spark boot path starts planning around BootInfo v2 seeds instead of only v1 fields.
- [x] remaining Spark-only roadmap language is removed from generated status outputs.

### M1 Spark Observatory Freeze

- Acceptance: `spark-observe.efi` captures real Spark facts and updates the BootInfo v2 draft with evidence.
- [x] `spark-observe.efi` exists and works in QEMU.
- [x] observatory output includes enough facts for BootInfo v2 accelerator/display/storage seeds.
- [x] observatory output is persisted in a structured artifact.
- [ ] real Spark observatory output is captured and compared with QEMU.
Supported proof flow now runs through `bash ./scripts/prepare-spark-hardware-bundle.sh spark-observe`, then `sudo -E bash ./scripts/install-spark-hardware-bundle.sh spark-observe`. Once the machine returns and stage-chain evidence is available, prefer `bash ./scripts/complete-spark-hardware-proof.sh /path/to/stage-chain-evidence.txt`, which now validates the returned stage-chain markers; `bash ./scripts/collect-spark-observe-report.sh` still exists for partial observatory-only returns. The milestone stays unchecked until that operator/root/reboot flow actually happens on Spark.

### M2 Spark Loader v2

- Acceptance: Spark UEFI boots the loader, exits boot services cleanly, and carries a v2-era fact model forward.
- [x] `novaaa64` loads typed stage1 and kernel payloads.
- [x] stage0 carries persistent digest and verification metadata.
- [x] stage0 and stage1 enforce the current payload load contract.
- [x] loader keeps an internal/transitional BootInfo v2 draft with one integrated/UMA accelerator seed while current bringup still uses BootInfo v1 as the primary contract.
- [x] loader exposes framebuffer, storage, and accelerator seed facts using the v2 model.
- [x] stage1 carries the transitional BootInfo v2 sidecar into the raw live kernel entry, and that entry resolves and records validated v2 facts while current bringup still uses BootInfo v1 as the primary contract.
- [ ] same loader path is proven on real Spark hardware.
Supported proof flow now runs through `bash ./scripts/prepare-spark-hardware-bundle.sh novaaa64`, then `sudo -E bash ./scripts/install-spark-hardware-bundle.sh novaaa64`, with optional `USE_BOOTNEXT=1` for one-time boot scheduling. Once the machine returns and stage-chain evidence is available, prefer `bash ./scripts/complete-spark-hardware-proof.sh /path/to/stage-chain-evidence.txt`, which now validates the returned stage-chain markers; `bash ./scripts/collect-novaaa64-loader-report.sh` still exists for partial loader-only returns. The milestone stays unchecked until that hardware boot actually happens and the operator brings back a passing loader handoff report plus stage0 -> stage1 -> kernel evidence from Spark.

### M3 Arm64 Kernel Core

- Acceptance: the Arm64 kernel brings up early mechanisms on Spark/QEMU under the portable architecture.
- [x] raw-entry kernel path exists.
- [x] early exception, page-table, allocator planning exists.
- [x] richer kernel runtime path is restored.
- [x] early framebuffer console exists beyond the scaffold.
- [ ] Spark hardware bring-up is proven.

### M4 Syscall ABI And IPC Core

- Acceptance: first user task runs with explicit capabilities and IPC.
- [x] syscall entry and exit.
- [x] capability checks.
- [ ] endpoint IPC.
- [ ] shared memory.
- [ ] `initd`.
Typed `init.capsule` bootstrap validation now exists, the loader snapshots embedded bootstrap-payload facts plus populated portable bootstrap user-window and loader-reserved bootstrap frame-arena descriptors into the transitional `BootInfo v2` sidecar when the embedded payload is valid, the kernel now binds capabilities and quota counts to a concrete current bootstrap-task object, early trace/yield/IPC syscalls enforce that task-boundary, the live bootstrap runtime now marks the first reserved bootstrap endpoint and shared-memory lanes ready when the capsule actually grants them and those lanes return real kernel-owned results in the QEMU probe, and the kernel can transfer control in place to the embedded bootstrap payload on a dedicated stack with a typed bootstrap context in `x0`, a logged unisolated `current-el-svc` current boundary plan, and a logged isolated `drop-to-el0`/`el0-svc` target boundary plan. That context still exposes a transitional same-EL bootstrap kernel-call gate, the default QEMU smoke lane now also proves a payload-originated same-EL `svc` return, and the AArch64 lower-EL sync vector now has a distinct bootstrap `svc` handler with a default-smoke dry-run that advances `ELR_EL1` for the future EL0 return path. The feature-gated raw EL0 diagnostic now proves no-MMU EL1-to-EL0 `eret`, lower-EL `svc` dispatch from `initd`, and return to EL0 spin, and the kernel now has tested EL0 mapping-readiness, backing-frame arena, page-table descriptor planners, payload/context/stack backing-frame population, transitional AArch64 translation-table construction, and TTBR/TCR/MAIR register planning for payload copy/rebase, context placement, and user stack sizing, with QEMU proving all three descriptor stages, backing-frame population, table construction, and register preparation as ready. The current QEMU evidence now shows the kernel-owned `svc`, pre-transfer `svc`, payload-originated `svc`, payload-originated `brk`, and raw EL0 probes all returning far enough for their diagnostic goals. Kernel-owned allocator integration, controlled TTBR/TCR/MAIR installation, MMU activation, endpoint routing beyond that reserved bootstrap slot, shared-memory policy beyond the first reserved bootstrap region, real EL0 user mappings/stacks, and a true isolated `initd` task still remain ahead.

### M5 CPU AI Runtime

- Acceptance: CPU-only model execution makes the OS useful before native GPU offload.
- [ ] tokenizer.
- [ ] tiny transformer CPU path.
- [ ] model capsule prototype.
- [ ] `cpuacceld`.

### M6 UMA Profile And Spark Local Shell

- Acceptance: UMA-aware memory policy, local shell, and persistent logs exist on Spark.
- [ ] UMA residency state machine.
- [ ] `memd` UMA policy.
- [ ] operator shell.
- [ ] persistent logs.
- [ ] local install/update path.

### M7 Fabric Contract Freeze v1

- Acceptance: the portable fabric contract is stable enough for later x86_64 and discrete lanes.
- [x] platform class enum exists.
- [x] transport and topology enums exist.
- [x] accelerator seed abstraction exists.
- [x] memory pool taxonomy exists.
- [x] queue classes exist.
- [ ] telemetry schema is documented and checked.
- [ ] sample service API is frozen.

### M8 Signed Boot And Capsules

- Acceptance: NovaOS has a real signing and rollback story.
- [ ] Secure Boot ownership path is documented.
- [ ] signed boot artifacts.
- [ ] signed capsules.
- [ ] rollback path.

### M9 x86_64 Observatory And Loader Skeleton

- Acceptance: a future PC/server lane exists as a bootable prototype.
- [ ] `pc-observe.efi`.
- [ ] `novax64` loader lane exists under NovaOS naming.
- [ ] x86_64 BootInfo v2 handoff in QEMU/OVMF.

### M10 x86_64 Kernel Bring-Up

- Acceptance: x86_64 boots the same kernel object model and syscall semantics.
- [x] `kernel/arch/x86_64/` placeholder exists.
- [ ] x86_64 entry path.
- [ ] timer and IRQ skeleton.
- [ ] framebuffer boot.
- [ ] syscall ABI parity with Arm64.

### M11 PCI / IOMMU / DMA Core

- Acceptance: the discrete-device lane has generic bus and DMA primitives.
- [x] `drivers/bus/pci/` placeholder exists.
- [ ] generic PCI enumeration.
- [ ] BAR mapping.
- [ ] PCI interrupt routing.
- [ ] DMA map primitives for userspace drivers.

### M12 NVIDIA Oracle Backend (Lab-Only)

- Acceptance: lab-only comparison tooling exists without becoming a shipping dependency.
- [ ] trace comparison tooling.
- [ ] oracle-backed fabric skeleton.
- [ ] build-time optional lab-only gating.

### M13 RTX Minimal Native Discrete Path

- Acceptance: NovaOS owns one discrete GPU as a native device object with basic telemetry and copy capability.
- [ ] discrete GPU ownership.
- [ ] telemetry path.
- [ ] memory pool reporting.
- [ ] minimal copy primitive.

### M14 GB10 Minimal Native Path

- Acceptance: integrated Spark accelerator ownership works natively.
- [ ] `gb10d`.
- [ ] minimal compute/copy/sync primitive.
- [ ] `acceld` integration.

### M15 Multi-Backend Scheduler v1

- Acceptance: placement logic spans CPU, UMA, and discrete backends cleanly.
- [ ] foreground/background placement.
- [ ] residency hints.
- [ ] queue arbitration.
- [ ] topology-aware policies for UMA and discrete.

### M16 Hopper / MIG / Partition Track

- Acceptance: partition objects and lease semantics exist behind the fabric contract.
- [ ] partition objects.
- [ ] lease model.
- [ ] Hopper discovery.
- [ ] peer-aware placement prototype.

### M17 Workstation Preview

- Acceptance: Spark remains stable while a developer-preview workstation lane exists.
- [ ] Spark stable path.
- [ ] RTX developer preview lane.
- [ ] package/update and shell parity.

### M18 Data-Center Lab Preview

- Acceptance: Hopper/H100 lab boot and partition scheduling prototypes exist.
- [ ] server-lab boot.
- [ ] partition scheduling prototype.
- [ ] peer-aware telemetry.

### M19 Release Candidate

- Acceptance: Spark first supported release, plus documented preview lanes.
- [ ] Spark first supported release.
- [ ] RTX developer preview lane.
- [ ] documented server-lab lane.
- [ ] signed artifacts and recovery path.

## Live Artifacts To Check

- `artifacts/reports/latest-status.txt`
- `artifacts/reports/latest-loop-status.txt`
- `artifacts/reports/latest-report.md`
- `artifacts/reports/latest-loop.log`
- `artifacts/reports/latest-loop.summary`

## Resume Discipline

- Keep this file and `docs/roadmap/live-status.md` in sync with on-disk reality.
- When a boot contract or milestone boundary changes, update the checklist before moving on.
- When real Spark facts are discovered, record them here and in the relevant milestone note.
