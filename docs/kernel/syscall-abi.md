# Syscall ABI v1

## Status

Drafted for the first M4 syscall-entry scaffold with current-task bootstrap capability gating.

The same-EL bootstrap kernel-call gate used by the current in-place `initd` handoff is a separate transitional bootstrap facility. This document still describes the `svc` ABI that NovaOS intends to use for the real syscall boundary.

## Current Arm64 convention

- `svc #imm` from EL0 is the syscall trap boundary.
- `x0..x5` carry arguments.
- `x6` carries request flags.
- `x7` is reserved.
- `x8` carries the syscall number.
- return `x0` carries the syscall status.
- return `x1` and `x2` carry result values.
- the return path advances `ELR_EL1` by one 4-byte `svc` instruction.

## Current scope

- `NOVA_SYSCALL_NOP` round-trips a successful empty syscall.
- The scaffold now binds syscall authorization to a concrete current bootstrap-task object seeded from `init.capsule`, not just a boot-global capability mask.
- `NOVA_SYSCALL_TRACE` now requires the bootstrap-task `BOOT_LOG` capability, and still proves argument and return-register flow while emitting a kernel trace line that includes the current bootstrap service identity.
- `NOVA_SYSCALL_YIELD` now requires the bootstrap-task `YIELD` capability, but still returns `Unsupported` until scheduling exists.
- `NOVA_SYSCALL_ENDPOINT_CALL` now requires the bootstrap-task `ENDPOINT_BOOTSTRAP` capability, treats `args[0]` as the reserved bootstrap endpoint index, rejects out-of-range indices as `INVALID_ARGS`, and when the live bootstrap task state reserves at least one endpoint slot the installed bootstrap runtime marks that lane ready and returns `Ok` with the slot and selector echoed back while emitting a kernel trace line. This is still a kernel-owned bootstrap lane, not full routing.
- `NOVA_SYSCALL_SHARED_MEMORY_MAP` now requires the bootstrap-task `SHARED_MEMORY_BOOTSTRAP` capability, treats `args[0]` as the reserved bootstrap region index, rejects out-of-range indices as `INVALID_ARGS`, and when the live bootstrap task state reserves at least one shared-memory region the installed bootstrap runtime marks that lane ready and returns `Ok` with the region and selector echoed back while emitting a kernel trace line. This is still a kernel-owned bootstrap lane, not full mapping policy.
- The default QEMU smoke path now builds the embedded `initd` payload with the payload-originated `bootstrap_svc_probe` feature, so the same-EL syscall trap path is proven in the normal boot lane after the transitional kernel-call gate.
- The AArch64 bootstrap vector table now has a distinct lower-EL AArch64 synchronous slot that routes to a lower-EL bootstrap `svc` handler. That handler uses the same task-bound dispatcher as the same-EL lane, but advances `ELR_EL1` by one `svc` instruction for the future EL0 return path. The default QEMU smoke path now also runs a kernel-owned lower-EL `svc` dry-run frame through that handler before entering `initd`, and the feature-gated raw EL0 diagnostic proves a no-MMU EL1-to-EL0 `eret` path can execute an `initd` `svc`, dispatch through that lower-EL handler, and return to EL0 spin.
- The remaining same-EL live exception diagnostics stay feature-gated as a kernel-owned `bootstrap_kernel_svc_probe` lane, a pre-transfer `bootstrap_pretransfer_svc_probe` lane, the payload `bootstrap_svc_probe` lane, and a payload `bootstrap_trap_probe` lane, capturable through `scripts/run-qemu-novaaa64-bootstrap-kernel-svc-diagnostic.sh`, `scripts/run-qemu-novaaa64-bootstrap-pretransfer-svc-diagnostic.sh`, `scripts/run-qemu-novaaa64-bootstrap-svc-diagnostic.sh`, and `scripts/run-qemu-novaaa64-bootstrap-trap-diagnostic.sh`. The current QEMU same-EL exception baseline returns end-to-end for all four lanes: kernel-owned `svc`, pre-transfer `svc`, payload-originated `svc`, and payload-originated `brk`. The kernel-owned `svc` probe keeps a scalar caller capture that now matches returned `x0/status`, `x1/value0`, and `x2/value1`.

## Current non-goals

- real EL0 task creation with user mappings and user-owned stacks
- capability object semantics beyond the narrow bootstrap policy mask
- endpoint routing or copy semantics beyond the first reserved bootstrap slot
- shared-memory mapping policy beyond the first reserved bootstrap region
- stable userspace service ABI beyond the initial syscall numbers
