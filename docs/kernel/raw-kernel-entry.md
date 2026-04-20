# Raw Kernel Entry

## Status

Accepted for the current boot-chain phase.

## Context

NovaOS now has a working QEMU chain:

- `novaaa64` loads `stage1.bin`, `kernel.bin`, and `init.capsule`
- `novaaa64` exits boot services cleanly
- `novaaa64` publishes explicit BootInfo presence flags for kernel digest and verification metadata
- `novaaa64` publishes a persistent `NovaVerificationInfoV1` record that stage1 and the kernel can resolve
- `stage1.bin` and `kernel.bin` now declare an explicit flat-binary load window and entry ABI in Nova Payload v1
- `stage1.bin` receives a `Stage1Plan`
- `stage1.bin` jumps into the loaded kernel image

The current kernel image is still a flat raw payload loaded at an arbitrary address.
That means the first executable kernel path must stay simple enough to avoid hidden relocation assumptions.

The earlier attempt to call the richer `kernel_entry()` path from the raw payload was not stable enough under this loading model.

## Decision

NovaOS will use a dedicated raw-entry kernel path until the loader grows beyond the current flat-binary handoff.

That raw-entry path must:

- accept `NovaBootInfoV1` as the primary bring-up contract
- resolve an optional `NovaBootInfoV2` sidecar when the staged payload ABI says it is present and record a compact set of validated v2 facts while `NovaBootInfoV1` remains primary
- validate the boot contract
- rely on the explicit payload load window and entry ABI instead of inferring them from header size alone
- resolve the staged verification object before trusting the loaded kernel image
- resolve the typed `init.capsule` bootstrap header before current bootstrap planning continues
- derive the first bootstrap task summary, current-task identity, and syscall authority mask from that typed `init.capsule` header while the broader capability model stays ahead
- prefer the `BootInfo v2` bootstrap payload descriptor for the first bootstrap task launch plan and first in-place bootstrap payload transfer when stage0 has already validated and snapshot that embedded payload
- enter the first bootstrap task through an explicit `BOOTSTRAP_TASK_V1` contract by sanitizing the live entry registers, hardening the in-place transfer around the EL1 bootstrap stack banks, computing both the current typed bootstrap boundary plan and the isolated EL0 target boundary plan, passing a typed bootstrap context in `x0`, and exposing a transitional same-EL bootstrap kernel-call gate through that typed context
- keep direct embedded bootstrap service payload parsing as the compatibility fallback until the real `initd` launch boundary lands
- prove the reserved post-exit memory-map window is readable
- build the early exception, page-table, and frame-allocator plans
- keep the early runtime and boot-console path concrete enough to stay proven under the raw image model
- carry a concrete early runtime far enough to prove boot-console output, the first syscall-entry scaffold, the first bootstrap payload transfer with a typed bootstrap context, and the first same-EL bootstrap kernel-call round-trip, while still avoiding claims of a finished task or privilege boundary

The richer shared runtime now runs behind the raw-entry validation shim, but the raw shim remains the boot-critical default until either:

- the kernel image is proven position-independent under the current raw loader, or
- stage0/stage1 grows a real executable image loader beyond the current flat-binary contract, and
- the verification-object contract is the default handoff for richer runtime setup

## Consequences

- The current boot chain stays stable while the kernel becomes incrementally more real.
- QEMU validation can keep proving `stage0 -> stage1 -> kernel` without regressing to a fake payload.
- The current QEMU proof still covers the same-EL in-place bootstrap transfer path with `current_el=1`, now logs that explicit unisolated `current-el-svc` boundary plan plus the intended isolated `drop-to-el0`/`el0-svc` target plan, proves a typed bootstrap kernel-call round-trip from `initd`, and proves a payload-originated same-EL `svc` return. Dedicated same-EL diagnostics still cover kernel-owned `svc`, pre-transfer `svc`, payload-originated `svc`, and payload-originated `brk` return end-to-end.
- The exception-vector foundation for a future EL0 `initd` boundary is now split: current-EL bootstrap `svc` keeps the existing no-ELR-advance return behavior, while the lower-EL AArch64 sync slot routes to a lower-EL `svc` handler that advances `ELR_EL1`. The default QEMU lane now proves that lower-EL handler with a kernel-owned dry-run frame before the real in-place bootstrap transfer, and the feature-gated raw EL0 diagnostic proves EL1-to-EL0 `eret`, lower-EL `svc` dispatch from `initd`, and return to EL0 spin under a no-MMU probe setup.
- The loader now publishes a portable BootInfo v2 bootstrap user-window policy when the embedded bootstrap payload descriptor is valid, so the kernel EL0 planner reports `bootstrap el0 mapping ready` instead of rejecting the old firmware-table placeholder. This is still planning evidence only; it does not allocate backing frames, build live page tables, or enable the MMU.
- The next step toward the full kernel path is still explicit and narrow: connect the ready mapping plan to allocator-backed frames and real page-table construction, then replace the no-MMU EL0 diagnostic shortcut with real user mappings/stacks and turn the current in-place bootstrap payload transfer plus typed bootstrap context into a real isolated `initd` task boundary.
