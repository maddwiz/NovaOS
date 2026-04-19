#ifndef NOVA_PAYLOAD_V1_H
#define NOVA_PAYLOAD_V1_H

#include <stdint.h>

#define NOVA_PAYLOAD_V1_MAGIC UINT64_C(0x3159415041564F4E)
#define NOVA_PAYLOAD_V1_VERSION UINT32_C(1)

enum nova_payload_kind_v1 {
    NOVA_PAYLOAD_KIND_STAGE1 = 1,
    NOVA_PAYLOAD_KIND_KERNEL = 2,
    NOVA_PAYLOAD_KIND_SERVICE = 3,
};

enum nova_payload_entry_abi_v1 {
    NOVA_PAYLOAD_ENTRY_ABI_STAGE1_PLAN = 1,
    NOVA_PAYLOAD_ENTRY_ABI_BOOTINFO = 2,
    NOVA_PAYLOAD_ENTRY_ABI_BOOTINFO_V2_SIDECAR = 3,
    NOVA_PAYLOAD_ENTRY_ABI_BOOTSTRAP_TASK_V1 = 4,
};

enum nova_payload_load_mode_v1 {
    NOVA_PAYLOAD_LOAD_MODE_FLAT_BINARY = 1,
};

struct nova_payload_header_v1 {
    uint64_t magic;
    uint32_t version;
    uint32_t kind;
    uint32_t header_size;
    uint32_t image_size;
    uint32_t load_offset;
    uint32_t load_size;
    uint32_t entry_offset;
    uint32_t entry_abi;
    uint32_t load_mode;
    uint32_t flags;
    uint32_t body_digest_algorithm;
    uint32_t body_digest_len;
    uint8_t body_digest[32];
};

#endif
