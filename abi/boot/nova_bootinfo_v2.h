#ifndef NOVA_BOOTINFO_V2_H
#define NOVA_BOOTINFO_V2_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define NOVA_BOOTINFO_V2_MAGIC UINT64_C(0x32424F4F5441564E) /* "NVATOOB2" */
#define NOVA_BOOTINFO_V2_VERSION UINT32_C(2)

enum NovaBootCpuArchitecture {
    NOVA_BOOT_CPU_ARCH_UNKNOWN = 0,
    NOVA_BOOT_CPU_ARCH_ARM64 = 1,
    NOVA_BOOT_CPU_ARCH_X86_64 = 2,
};

enum NovaPlatformClass {
    NOVA_PLATFORM_CLASS_UNKNOWN = 0,
    NOVA_PLATFORM_CLASS_SPARK_UMA = 1,
    NOVA_PLATFORM_CLASS_PCIE_SINGLE = 2,
    NOVA_PLATFORM_CLASS_PCIE_MULTI = 3,
    NOVA_PLATFORM_CLASS_FABRIC_PARTITIONED = 4,
};

enum NovaMemoryTopologyClass {
    NOVA_MEMORY_TOPOLOGY_UNKNOWN = 0,
    NOVA_MEMORY_TOPOLOGY_UMA = 1,
    NOVA_MEMORY_TOPOLOGY_DISCRETE = 2,
    NOVA_MEMORY_TOPOLOGY_NVLINK = 3,
    NOVA_MEMORY_TOPOLOGY_MIG = 4,
};

enum NovaAccelTransport {
    NOVA_ACCEL_TRANSPORT_UNKNOWN = 0,
    NOVA_ACCEL_TRANSPORT_INTEGRATED = 1,
    NOVA_ACCEL_TRANSPORT_PLATFORM = 2,
    NOVA_ACCEL_TRANSPORT_PCI = 3,
    NOVA_ACCEL_TRANSPORT_FABRIC = 4,
};

enum NovaAccelTopologyHint {
    NOVA_ACCEL_TOPOLOGY_UNKNOWN = 0,
    NOVA_ACCEL_TOPOLOGY_UMA = 1,
    NOVA_ACCEL_TOPOLOGY_DISCRETE = 2,
    NOVA_ACCEL_TOPOLOGY_PARTITIONABLE = 3,
    NOVA_ACCEL_TOPOLOGY_LINKED = 4,
};

typedef struct NovaFramebufferDescriptorV1 {
    uint64_t base;
    uint32_t width;
    uint32_t height;
    uint32_t stride;
    uint32_t format;
} NovaFramebufferDescriptorV1;

typedef struct NovaDisplayPathDescriptorV1 {
    uint64_t device_path_ptr;
    uint32_t device_path_len;
    uint32_t flags;
} NovaDisplayPathDescriptorV1;

typedef struct NovaStorageSeedV1 {
    uint64_t device_path_ptr;
    uint32_t device_path_len;
    uint32_t flags;
} NovaStorageSeedV1;

typedef struct NovaNetworkSeedV1 {
    uint64_t device_path_ptr;
    uint32_t device_path_len;
    uint32_t flags;
} NovaNetworkSeedV1;

typedef struct NovaBootstrapPayloadDescriptorV1 {
    uint64_t image_ptr;
    uint64_t image_len;
    uint64_t load_base;
    uint64_t load_size;
    uint64_t entry_point;
} NovaBootstrapPayloadDescriptorV1;

typedef struct NovaBootstrapUserWindowDescriptorV1 {
    uint64_t base;
    uint64_t len;
    uint64_t stack_size;
    uint32_t page_size;
    uint32_t flags; /* Reserved; must be zero in the current draft. */
} NovaBootstrapUserWindowDescriptorV1;

typedef struct NovaAccelMmioWindowV1 {
    uint64_t base;
    uint64_t len;
    uint32_t flags;
    uint32_t reserved0;
} NovaAccelMmioWindowV1;

typedef struct NovaAccelSeedV1 {
    uint16_t vendor_id;
    uint16_t device_id;
    uint32_t class_code;
    uint16_t transport;
    uint16_t topology_hint;
    uint16_t memory_topology;
    uint8_t mmio_window_count;
    uint8_t interrupt_hint;
    uint16_t reserved0;
    uint64_t raw_table_ptr;
    uint32_t raw_table_len;
    uint32_t reserved1;
    NovaAccelMmioWindowV1 mmio_windows[4];
} NovaAccelSeedV1;

typedef struct NovaBootInfoV2 {
    uint64_t magic;
    uint32_t version;
    uint32_t flags;

    uint16_t cpu_arch;
    uint16_t platform_class;
    uint16_t memory_topology_class;
    uint8_t secure_boot_state;
    uint8_t boot_source;
    uint8_t current_el;
    uint8_t reserved0;
    uint16_t reserved1;

    uint64_t firmware_vendor_ptr;
    uint32_t firmware_revision;
    uint32_t reserved2;

    uint64_t memory_map_ptr;
    uint32_t memory_map_entries;
    uint32_t memory_map_desc_size;
    uint64_t config_tables_ptr;
    uint32_t config_table_count;
    uint32_t reserved3;

    uint64_t acpi_rsdp_ptr;
    uint64_t dtb_ptr;
    uint64_t smbios_ptr;
    uint64_t vendor_tables_ptr;
    uint32_t vendor_table_count;
    uint32_t reserved4;

    NovaFramebufferDescriptorV1 framebuffer;
    NovaDisplayPathDescriptorV1 display_path;

    uint64_t storage_seeds_ptr;
    uint32_t storage_seed_count;
    uint32_t reserved5;
    uint64_t network_seeds_ptr;
    uint32_t network_seed_count;
    uint32_t reserved6;
    uint64_t accel_seeds_ptr;
    uint32_t accel_seed_count;
    uint32_t reserved7;

    uint64_t init_capsule_ptr;
    uint64_t init_capsule_len;
    uint64_t loader_log_ptr;
    uint64_t loader_log_len;
    uint64_t kernel_image_hash_ptr;
    uint64_t loader_image_hash_ptr;
    uint64_t boot_counter;
    uint64_t observatory_hash_ptr;
    NovaBootstrapPayloadDescriptorV1 bootstrap_payload;
    NovaBootstrapUserWindowDescriptorV1 bootstrap_user_window;
} NovaBootInfoV2;

static inline void nova_bootinfo_v2_init(NovaBootInfoV2 *info) {
    if (info == NULL) {
        return;
    }

    *info = (NovaBootInfoV2){
        .magic = NOVA_BOOTINFO_V2_MAGIC,
        .version = NOVA_BOOTINFO_V2_VERSION,
    };
}

_Static_assert(sizeof(NovaBootstrapPayloadDescriptorV1) == 40, "unexpected bootstrap payload descriptor layout");
_Static_assert(sizeof(NovaBootstrapUserWindowDescriptorV1) == 32, "unexpected bootstrap user window descriptor layout");
_Static_assert(sizeof(NovaBootInfoV2) == 344, "NovaBootInfoV2 layout must stay stable");
_Static_assert(offsetof(NovaBootInfoV2, bootstrap_payload) == 272, "unexpected NovaBootInfoV2 layout");
_Static_assert(offsetof(NovaBootInfoV2, bootstrap_user_window) == 312, "unexpected NovaBootInfoV2 layout");

#ifdef __cplusplus
}
#endif

#endif /* NOVA_BOOTINFO_V2_H */
