# NovaVerificationInfoV1 Layout Notes

This document mirrors `nova_verification_v1.h` for Rust and other FFI consumers.

## Purpose

`NovaVerificationInfoV1` is the first explicit boot-verification record for NovaOS.
It records facts established by stage0 before control transfers into stage1:

- whether `stage1.bin` was present and validated as a typed payload
- whether `kernel.bin` was present and validated as a typed payload
- whether a persistent kernel-image digest object was published and matched the staged kernel image
- whether an init capsule was staged

## Rules

- Use `#[repr(C)]` in Rust.
- Treat the flags as observed loader facts, not policy decisions.
- If a `*_VERIFIED` flag is set, the matching `*_PRESENT` flag must also be set.
- `stage1_image_size` and `kernel_image_size` are the staged wrapped image sizes in bytes.

## Rust mirror

```rust
#[repr(C)]
pub struct NovaVerificationInfoV1 {
    pub magic: u64,
    pub version: u32,
    pub flags: u32,
    pub stage1_image_size: u64,
    pub kernel_image_size: u64,
}
```

## BootInfo integration

- `NovaBootInfoV1.flags` uses `FLAG_HAS_VERIFICATION_INFO` when `verification_info_ptr` is valid.
- `verification_info_ptr` points to a `NovaVerificationInfoV1` object in loader-owned memory.
