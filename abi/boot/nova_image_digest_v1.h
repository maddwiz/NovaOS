#ifndef NOVA_IMAGE_DIGEST_V1_H
#define NOVA_IMAGE_DIGEST_V1_H

#include <stdint.h>

#define NOVA_IMAGE_DIGEST_V1_MAGIC UINT64_C(0x31545347444D564E)

enum nova_digest_algorithm_v1 {
    NOVA_DIGEST_ALGORITHM_SHA256 = 1,
};

struct nova_image_digest_v1 {
    uint64_t magic;
    uint32_t algorithm;
    uint32_t byte_len;
    uint8_t bytes[32];
};

#endif
