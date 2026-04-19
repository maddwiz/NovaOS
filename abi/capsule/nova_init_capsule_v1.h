#ifndef NOVA_INIT_CAPSULE_V1_H
#define NOVA_INIT_CAPSULE_V1_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define NOVA_INIT_CAPSULE_V1_MAGIC UINT64_C(0x54494E4941564F4E) /* "NOVAINIT" */
#define NOVA_INIT_CAPSULE_V1_VERSION UINT32_C(1)
#define NOVA_INIT_CAPSULE_SERVICE_NAME_LEN 16

enum NovaInitCapsuleCapabilityV1 {
    NOVA_INIT_CAPSULE_CAP_BOOT_LOG = UINT64_C(1) << 0,
    NOVA_INIT_CAPSULE_CAP_YIELD = UINT64_C(1) << 1,
    NOVA_INIT_CAPSULE_CAP_ENDPOINT_BOOTSTRAP = UINT64_C(1) << 2,
    NOVA_INIT_CAPSULE_CAP_SHARED_MEMORY_BOOTSTRAP = UINT64_C(1) << 3,
};

typedef struct NovaInitCapsuleHeaderV1 {
    uint64_t magic;
    uint32_t version;
    uint32_t header_size;
    uint32_t total_size;
    uint32_t flags;
    uint64_t requested_capabilities;
    uint32_t endpoint_slots;
    uint32_t shared_memory_regions;
    uint8_t service_name[NOVA_INIT_CAPSULE_SERVICE_NAME_LEN];
    uint8_t reserved[8];
} NovaInitCapsuleHeaderV1;

static inline void nova_init_capsule_v1_init(
    NovaInitCapsuleHeaderV1 *header,
    const uint8_t service_name[NOVA_INIT_CAPSULE_SERVICE_NAME_LEN],
    uint64_t requested_capabilities,
    uint32_t endpoint_slots,
    uint32_t shared_memory_regions
) {
    size_t index;

    if (header == NULL) {
        return;
    }

    *header = (NovaInitCapsuleHeaderV1){
        .magic = NOVA_INIT_CAPSULE_V1_MAGIC,
        .version = NOVA_INIT_CAPSULE_V1_VERSION,
        .header_size = sizeof(NovaInitCapsuleHeaderV1),
        .total_size = sizeof(NovaInitCapsuleHeaderV1),
        .requested_capabilities = requested_capabilities,
        .endpoint_slots = endpoint_slots,
        .shared_memory_regions = shared_memory_regions,
    };

    if (service_name == NULL) {
        return;
    }

    for (index = 0; index < NOVA_INIT_CAPSULE_SERVICE_NAME_LEN; ++index) {
        header->service_name[index] = service_name[index];
    }
}

_Static_assert(sizeof(NovaInitCapsuleHeaderV1) == 64, "NovaInitCapsuleHeaderV1 layout must stay stable");
_Static_assert(offsetof(NovaInitCapsuleHeaderV1, service_name) == 40, "unexpected NovaInitCapsuleHeaderV1 layout");

#ifdef __cplusplus
}
#endif

#endif /* NOVA_INIT_CAPSULE_V1_H */
