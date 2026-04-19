# NovaInitCapsule v1 Layout Notes

This document mirrors `nova_init_capsule_v1.h` for Rust and other FFI consumers.

## Rules

- `init.capsule` is now a typed bootstrap object instead of an opaque text placeholder.
- `total_size` may equal `header_size` for a metadata-only capsule, or it may include an embedded bootstrap service payload in the capsule body.
- `service_name` is a lowercase ASCII bootstrap service name stored as a NUL-padded fixed array.
- `requested_capabilities` is append-only and currently reserves the first early-bootstrap authority bits.
- the default generated capsule now reserves one bootstrap endpoint slot and one bootstrap shared-memory region.

## Rust mirror

```rust
#[repr(C)]
pub struct NovaInitCapsuleHeaderV1 {
    pub magic: u64,
    pub version: u32,
    pub header_size: u32,
    pub total_size: u32,
    pub flags: u32,
    pub requested_capabilities: u64,
    pub endpoint_slots: u32,
    pub shared_memory_regions: u32,
    pub service_name: [u8; 16],
    pub reserved: [u8; 8],
}
```

## Current scope

- `initd` is the default bootstrap service name.
- `NOVA_INIT_CAPSULE_CAP_BOOT_LOG | NOVA_INIT_CAPSULE_CAP_ENDPOINT_BOOTSTRAP | NOVA_INIT_CAPSULE_CAP_SHARED_MEMORY_BOOTSTRAP` is the current default requested capability mask.
- The default generated capsule now embeds a wrapped bootstrap service payload image for `initd`.
- The capsule body, when present, must be a `SERVICE`/`BOOTSTRAP_TASK_V1` Nova Payload image.
- The capsule is validated by stage0/stage1/kernel bring-up before current execution continues.
