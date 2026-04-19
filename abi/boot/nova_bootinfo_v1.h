#ifndef NOVA_BOOTINFO_V1_H
#define NOVA_BOOTINFO_V1_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define NOVA_BOOTINFO_MAGIC UINT64_C(0x4E4F5641424F4F54) /* "NOVABOOT" */
#define NOVA_BOOTINFO_V1_VERSION UINT32_C(1)

enum NovaBootSecureBootState {
    NOVA_BOOT_SECURE_BOOT_UNKNOWN = 0,
    NOVA_BOOT_SECURE_BOOT_DISABLED = 1,
    NOVA_BOOT_SECURE_BOOT_ENABLED = 2,
};

enum NovaBootSource {
    NOVA_BOOT_SOURCE_UNKNOWN = 0,
    NOVA_BOOT_SOURCE_USB = 1,
    NOVA_BOOT_SOURCE_BOOT_OPTION = 2,
    NOVA_BOOT_SOURCE_PXE = 3,
    NOVA_BOOT_SOURCE_INTERNAL_NVME = 4,
};

enum NovaFramebufferFormat {
    NOVA_FRAMEBUFFER_FORMAT_UNKNOWN = 0,
    NOVA_FRAMEBUFFER_FORMAT_RGBX8888 = 1,
    NOVA_FRAMEBUFFER_FORMAT_BGRX8888 = 2,
};

enum NovaBootFlags {
    NOVA_BOOT_FLAG_HAS_ACPI_RSDP = UINT32_C(1) << 0,
    NOVA_BOOT_FLAG_HAS_DTB = UINT32_C(1) << 1,
    NOVA_BOOT_FLAG_HAS_SMBIOS = UINT32_C(1) << 2,
    NOVA_BOOT_FLAG_HAS_FRAMEBUFFER = UINT32_C(1) << 3,
    NOVA_BOOT_FLAG_HAS_LOADER_LOG = UINT32_C(1) << 4,
    NOVA_BOOT_FLAG_HAS_KERNEL_IMAGE_DIGEST = UINT32_C(1) << 5,
    NOVA_BOOT_FLAG_HAS_VERIFICATION_INFO = UINT32_C(1) << 6,
};

typedef struct NovaBootInfoV1 {
    uint64_t magic;
    uint32_t version;
    uint32_t flags;

    uint64_t firmware_vendor_ptr;
    uint32_t firmware_revision;
    uint8_t secure_boot_state;
    uint8_t boot_source;
    uint8_t current_el;
    uint8_t reserved0;

    uint64_t memory_map_ptr;
    uint32_t memory_map_entries;
    uint32_t memory_map_desc_size;
    uint64_t config_tables_ptr;
    uint32_t config_table_count;
    uint32_t reserved1;

    uint64_t acpi_rsdp_ptr;
    uint64_t dtb_ptr;
    uint64_t smbios_ptr;

    uint64_t framebuffer_base;
    uint32_t framebuffer_width;
    uint32_t framebuffer_height;
    uint32_t framebuffer_stride; /* PixelsPerScanLine, not bytes */
    uint32_t framebuffer_format;

    uint64_t init_capsule_ptr;
    uint64_t init_capsule_len;
    uint64_t kernel_image_hash_ptr;
    uint64_t loader_log_ptr;
    uint64_t verification_info_ptr;
} NovaBootInfoV1;

static inline void nova_bootinfo_v1_init(NovaBootInfoV1 *info) {
    if (info == NULL) {
        return;
    }

    *info = (NovaBootInfoV1){
        .magic = NOVA_BOOTINFO_MAGIC,
        .version = NOVA_BOOTINFO_V1_VERSION,
    };
}

_Static_assert(sizeof(NovaBootInfoV1) == 152, "NovaBootInfoV1 layout must stay stable");
_Static_assert(offsetof(NovaBootInfoV1, framebuffer_base) == 88, "unexpected NovaBootInfoV1 layout");
_Static_assert(offsetof(NovaBootInfoV1, verification_info_ptr) == 144, "unexpected NovaBootInfoV1 layout");

#ifdef __cplusplus
}
#endif

#endif /* NOVA_BOOTINFO_V1_H */
