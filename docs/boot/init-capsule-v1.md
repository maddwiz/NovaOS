# Init Capsule v1

`init.capsule` is the first typed bootstrap object carried by NovaOS boot media.

## Current contract

- The file uses the fixed `NovaInitCapsuleHeaderV1` layout in [nova_init_capsule_v1.h](/home/nova/NovaOS/abi/capsule/nova_init_capsule_v1.h).
- The default generated capsule names the bootstrap service `initd`.
- The default generated capsule currently requests `NOVA_INIT_CAPSULE_CAP_BOOT_LOG | NOVA_INIT_CAPSULE_CAP_ENDPOINT_BOOTSTRAP | NOVA_INIT_CAPSULE_CAP_SHARED_MEMORY_BOOTSTRAP`.
- The default generated capsule now embeds a wrapped `SERVICE`/`BOOTSTRAP_TASK_V1` payload image for `initd` in the capsule body.
- The default generated capsule now reserves one bootstrap endpoint slot and one bootstrap shared-memory region.

## Current validation path

- Host tooling builds the capsule through `scripts/build-init-capsule.sh`.
- Stage0 keeps carrying the capsule by pointer and length in BootInfo.
- Stage0 now also snapshots an explicit embedded bootstrap payload descriptor into the transitional `BootInfo v2` sidecar whenever the capsule body carries a valid wrapped `BOOTSTRAP_TASK_V1` image.
- Stage1 rejects an invalid typed capsule or embedded bootstrap payload before the live handoff continues.
- The raw Arm64 kernel resolves the typed capsule header, records a compact bring-up summary from it, and materializes the first current bootstrap-task object with authority and resource quotas from that header contract.
- The first reserved bootstrap endpoint slot now also enables the installed bootstrap syscall runtime when the capsule requests `ENDPOINT_BOOTSTRAP`, and the QEMU probe exercises that live lane so endpoint bootstrap IPC is no longer only an ABI placeholder.
- The first reserved bootstrap shared-memory region now also enables the installed bootstrap syscall runtime when the capsule requests `SHARED_MEMORY_BOOTSTRAP`, and the QEMU probe exercises that live lane with a kernel-owned echo result.
- When the embedded bootstrap payload is usable, the raw kernel now prefers the `BootInfo v2` bootstrap payload descriptor, derives a first bootstrap-task launch plan from that staged image metadata, syncs the instruction cache for that image, materializes a typed bootstrap-task context from the validated capsule header, computes both the current bootstrap boundary plan and the isolated EL0 target boundary plan, and transfers control in place onto a dedicated bootstrap-task stack.
- The raw live kernel still keeps the capsule header as the bootstrap authority source, and only falls back to direct payload parsing when the `BootInfo v2` sidecar does not carry the embedded payload descriptor.
- The current `BOOTSTRAP_TASK_V1` entry now receives that typed bootstrap context in `x0`, exposes a transitional same-EL bootstrap kernel-call gate for the first `initd -> kernel -> initd` round-trip, and keeps the rest of the early argument registers sanitized for a cleaner boundary.
- The default QEMU smoke path now also builds the embedded `initd` payload with the payload-originated `svc` probe, proving the same-EL syscall trap path returns after the transitional kernel-call gate.
- The lower-EL AArch64 sync vector now routes to a distinct bootstrap `svc` handler that advances `ELR_EL1`, and the default QEMU smoke path proves that handler with a kernel-owned dry-run frame before the real in-place transfer. This gives the later EL0 `initd` probe a separate return path from the current same-EL bootstrap lane.
- The `BOOTSTRAP_TASK_V1` payload can also be rebuilt with the `bootstrap_trap_probe` feature through `scripts/run-qemu-novaaa64-bootstrap-trap-diagnostic.sh`; that opt-in diagnostic records how far the same-EL payload trap lane gets without weakening the default smoke baseline.
- That transfer still runs under the current privileged raw-kernel environment; the logged current boundary plan is explicitly unisolated and uses `current-el-svc`, while the separate target plan records the intended isolated `drop-to-el0`/`el0-svc` boundary. This is still a first bootstrap payload handoff, not a finished isolated user-task model.

## Current non-goals

- launching a real user task
- full capability object enforcement
- endpoint routing beyond the first reserved bootstrap slot
- shared-memory mapping policy beyond the first reserved bootstrap region

This is the first enforced bootstrap task boundary, not a finished `initd` lane.
