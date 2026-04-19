#ifndef NOVA_VERIFICATION_V1_H
#define NOVA_VERIFICATION_V1_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define NOVA_VERIFICATION_MAGIC UINT64_C(0x3146495245564F4E) /* "NOVERIF1" */
#define NOVA_VERIFICATION_V1_VERSION UINT32_C(1)

enum NovaVerificationFlags {
    NOVA_VERIFICATION_FLAG_STAGE1_PAYLOAD_PRESENT = UINT32_C(1) << 0,
    NOVA_VERIFICATION_FLAG_STAGE1_PAYLOAD_VERIFIED = UINT32_C(1) << 1,
    NOVA_VERIFICATION_FLAG_KERNEL_PAYLOAD_PRESENT = UINT32_C(1) << 2,
    NOVA_VERIFICATION_FLAG_KERNEL_PAYLOAD_VERIFIED = UINT32_C(1) << 3,
    NOVA_VERIFICATION_FLAG_KERNEL_DIGEST_PRESENT = UINT32_C(1) << 4,
    NOVA_VERIFICATION_FLAG_KERNEL_DIGEST_VERIFIED = UINT32_C(1) << 5,
    NOVA_VERIFICATION_FLAG_INIT_CAPSULE_PRESENT = UINT32_C(1) << 6,
};

typedef struct NovaVerificationInfoV1 {
    uint64_t magic;
    uint32_t version;
    uint32_t flags;
    uint64_t stage1_image_size;
    uint64_t kernel_image_size;
} NovaVerificationInfoV1;

static inline void nova_verification_v1_init(NovaVerificationInfoV1 *info) {
    if (info == NULL) {
        return;
    }

    *info = (NovaVerificationInfoV1){
        .magic = NOVA_VERIFICATION_MAGIC,
        .version = NOVA_VERIFICATION_V1_VERSION,
    };
}

_Static_assert(
    sizeof(NovaVerificationInfoV1) == 32,
    "NovaVerificationInfoV1 layout must stay stable"
);

#ifdef __cplusplus
}
#endif

#endif /* NOVA_VERIFICATION_V1_H */
