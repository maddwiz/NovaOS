# BootInfo v1

Superseded in architecture by [BootInfo v2 Draft](/home/nova/NovaOS/docs/boot/bootinfo-v2.md).

BootInfo v1 is the first loader-to-kernel handoff contract for NovaOS.

The rule is simple: BootInfo carries facts, not guesses.

## Required fields

```c
struct BootInfo {
    u64 magic;
    u32 version;
    u32 flags;

    u64 firmware_vendor_ptr;
    u32 firmware_revision;
    u8 secure_boot_state;
    u8 boot_source;
    u8 current_el;
    u8 reserved0;

    u64 memory_map_ptr;
    u32 memory_map_entries;
    u32 memory_map_desc_size;
    u64 config_tables_ptr;
    u32 config_table_count;
    u32 reserved1;

    u64 acpi_rsdp_ptr;
    u64 dtb_ptr;
    u64 smbios_ptr;

    u64 framebuffer_base;
    u32 framebuffer_width;
    u32 framebuffer_height;
    u32 framebuffer_stride;
    u32 framebuffer_format;

    u64 init_capsule_ptr;
    u64 init_capsule_len;
    u64 kernel_image_hash_ptr;
    u64 loader_log_ptr;
    u64 verification_info_ptr;
};
```

## Contract rules

- `acpi_rsdp_ptr` may be zero.
- `dtb_ptr` may be zero.
- the kernel must not infer platform topology from missing tables.
- the loader must reserve its own memory before `ExitBootServices()`.
- framebuffer state must be passed through even if only simple text output is available.
- `framebuffer_stride` is pixels per scan line, matching UEFI GOP `PixelsPerScanLine`, not a byte count.
- `init_capsule_ptr` and `init_capsule_len` point to the typed `InitCapsule v1` bootstrap object when the loader stages one, including any embedded bootstrap service payload carried in the capsule body.
- `kernel_image_hash_ptr` points to a `NovaImageDigestV1` object when `FLAG_HAS_KERNEL_IMAGE_DIGEST` is set.
- `verification_info_ptr` points to a `NovaVerificationInfoV1` object when `FLAG_HAS_VERIFICATION_INFO` is set.

## Early uses

The first kernel uses BootInfo to:
- identify the firmware handoff
- print platform identity
- initialize early logging
- install exception vectors
- build initial page tables
- map the framebuffer

## Non-goals for v1

- device enumeration policy
- filesystem semantics
- capability object tables
- runtime driver metadata
- any native GB10 programming model
