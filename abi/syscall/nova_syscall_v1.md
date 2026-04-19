# NovaSyscall v1 Layout Notes

This document mirrors `nova_syscall_v1.h` for Rust and other FFI consumers.

## Rules

- The v1 syscall contract is register-oriented on Arm64, but the request/result structs are the stable typed mirror for tests, traces, and future non-raw entry paths.
- `number` stays append-only unless the major ABI version changes.
- `flags` are request- or result-specific and default to zero unless a syscall definition says otherwise.
- Endpoint, capability, and shared-memory semantics are intentionally not frozen by this file yet.

## Rust mirror

```rust
pub const NOVA_SYSCALL_ARG_COUNT: usize = 6;

#[repr(C)]
pub struct NovaSyscallRequestV1 {
    pub number: u32,
    pub flags: u32,
    pub args: [u64; NOVA_SYSCALL_ARG_COUNT],
}

#[repr(C)]
pub struct NovaSyscallResultV1 {
    pub status: u32,
    pub flags: u32,
    pub value0: u64,
    pub value1: u64,
}
```

## Current Arm64 register convention

- `x0..x5`: syscall arguments
- `x6`: request flags
- `x7`: reserved for future expansion
- `x8`: syscall number
- `svc #imm`: trap into the kernel
- return `x0`: status
- return `x1`: value0
- return `x2`: value1

## Current scope

- `NOVA_SYSCALL_NOP` and `NOVA_SYSCALL_TRACE` exist as the first round-trip scaffolding.
- `NOVA_SYSCALL_YIELD`, `NOVA_SYSCALL_ENDPOINT_CALL`, and `NOVA_SYSCALL_SHARED_MEMORY_MAP` reserve numbers for the next M4 steps.
