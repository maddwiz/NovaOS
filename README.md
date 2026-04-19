# NovaOS

NovaOS is a from-scratch operating system scaffold for NVIDIA compute platforms, with Spark as the first truth platform.

The current scope is intentionally disciplined:
- keep Spark / GB10 as the first hardware oracle
- design the core to survive later x86_64 workstation and server ports
- own the system from the UEFI handoff upward
- define a native boot path, kernel ABI, service model, and fabric contract
- treat unified memory, discrete VRAM, and fabric-connected memory as first-class system resources

This repository starts with the M0-M17 roadmap:
- `spark-observe.efi` and the platform observatory notes
- stage0 loader and BootInfo direction notes
- Nova Payload v1 headers with explicit load-window, entry-ABI, and body-digest verification for typed `stage1.bin` and `kernel.bin` artifacts
- explicit BootInfo presence flags and a persistent `NovaVerificationInfoV1` boot-verification record for staged payload checks
- the M0-M2 boot-chain foundation that reaches QEMU kernel entry
- the portability and accelerator-fabric milestones that prepare the codebase for later RTX and Hopper-class ports

The current QEMU handoff chain reaches stage0 post-exit, stage1 entry, kernel entry, and the first in-place `initd` payload transfer. The Arm64 kernel path now restores validated raw bringup state, binds the typed `init.capsule` contract to a concrete current bootstrap-task object with capabilities and quota counts, emits early boot-console output, and carries a first syscall-entry scaffold while the broader runtime remains intentionally small and boot-contract-first. Stage0 and stage1 still validate the embedded `initd` service payload before handoff continues, the loader snapshots that embedded bootstrap payload into the `BootInfo v2` sidecar, and the raw live kernel now reuses that explicit launch descriptor to transfer control onto a dedicated bootstrap-task stack with a typed bootstrap context in `x0` while hardening the live handoff around EL1 stack selection for in-place bringup. The kernel payload wrapper now also pads the kernel body to an alignment-safe `load_offset`, so stage1's existing in-place `load_base` contract no longer lands the live vector table on a misaligned address. That same typed context still carries a transitional same-EL bootstrap kernel-call gate, so the current QEMU lane proves a real `initd -> kernel -> initd` round-trip without claiming that the final syscall or privilege boundary exists yet. Opt-in same-EL exception diagnostics now exist as a kernel-owned `bootstrap_kernel_svc_probe` lane plus pre-transfer and payload `bootstrap_svc_probe` lanes plus a payload `bootstrap_trap_probe` lane. Those diagnostics are still kept separate from the default green baseline, but the blocker is narrower now: the kernel-owned `svc`, pre-transfer `svc`, and payload `svc` probes all reach vector pre-stack entry, vector entry, the live handler, and vector return, while the payload `brk` probe now returns end-to-end. The current kernel-owned `svc` probe now shows `x0/status` returning as `Ok`, and the new pre-`eret` capture still shows `x1/x2` correct in-frame and after restore, but the raw caller-side observation still sees `x1/value0` collapse to zero while `x2/value1` stays correct. The current QEMU lane still reports `current_el=1`. This is still not a finished user-task or EL0 boundary.

The 24/7 validation loop leaves stable artifacts in `artifacts/reports/`, including `latest-report.md`, `latest-loop.log`, and `latest-status.txt` for quick inspection without chasing timestamps.

For seamless resume from another Codex session, start with:
- `docs/roadmap/START-HERE.md`
- `docs/roadmap/live-status.md`
- `docs/roadmap/master-roadmap-checklist.md`

The codebase is expected to grow in this order:
1. observability
2. custom loader
3. tiny kernel
4. exception and memory management
5. capability-based services
6. portable accelerator fabric

Keep the scope strict. NovaOS is not a general-purpose desktop, not a Linux distribution, and not a permanent compatibility shim for vendor stacks.
