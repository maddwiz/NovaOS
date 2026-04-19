# NovaBootInfoV1 Layout Notes

This document mirrors `nova_bootinfo_v1.h` for Rust and other FFI consumers.

## Rules

- Use `#[repr(C)]` in Rust.
- Store addresses and firmware pointers as `u64`.
- Do not assume ACPI, DT, or SMBIOS are present unless the matching flag is set.
- Treat `secure_boot_state` and `boot_source` as facts reported by the loader, not policy decisions.
- Treat `framebuffer_stride` as pixels per scan line, matching UEFI GOP `PixelsPerScanLine`.
- Keep the struct append-only unless the major ABI version changes.

## Rust mirror

```rust
#[repr(C)]
pub struct NovaBootInfoV1 {
    pub magic: u64,
    pub version: u32,
    pub flags: u32,

    pub firmware_vendor_ptr: u64,
    pub firmware_revision: u32,
    pub secure_boot_state: u8,
    pub boot_source: u8,
    pub current_el: u8,
    pub reserved0: u8,

    pub memory_map_ptr: u64,
    pub memory_map_entries: u32,
    pub memory_map_desc_size: u32,
    pub config_tables_ptr: u64,
    pub config_table_count: u32,
    pub reserved1: u32,

    pub acpi_rsdp_ptr: u64,
    pub dtb_ptr: u64,
    pub smbios_ptr: u64,

    pub framebuffer_base: u64,
    pub framebuffer_width: u32,
    pub framebuffer_height: u32,
    pub framebuffer_stride: u32,
    pub framebuffer_format: u32,

    pub init_capsule_ptr: u64,
    pub init_capsule_len: u64,
    pub kernel_image_hash_ptr: u64,
    pub loader_log_ptr: u64,
    pub verification_info_ptr: u64,
}
```

## Compatibility note

The `flags` field is the only place where the loader should indicate optional data
presence. The actual pointers may be zero even when the struct is valid.

## Current pointer use

- `kernel_image_hash_ptr` points to a `NovaImageDigestV1` object when `FLAG_HAS_KERNEL_IMAGE_DIGEST` is set.
- `verification_info_ptr` points to a `NovaVerificationInfoV1` object when `FLAG_HAS_VERIFICATION_INFO` is set.
