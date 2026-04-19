#ifndef NOVA_SYSCALL_V1_H
#define NOVA_SYSCALL_V1_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define NOVA_SYSCALL_V1_VERSION UINT32_C(1)
#define NOVA_SYSCALL_ARG_COUNT 6

enum NovaSyscallNumberV1 {
    NOVA_SYSCALL_NOP = 0,
    NOVA_SYSCALL_TRACE = 1,
    NOVA_SYSCALL_YIELD = 2,
    NOVA_SYSCALL_ENDPOINT_CALL = 3,
    NOVA_SYSCALL_SHARED_MEMORY_MAP = 4,
};

enum NovaSyscallStatusV1 {
    NOVA_SYSCALL_STATUS_OK = 0,
    NOVA_SYSCALL_STATUS_UNKNOWN = 1,
    NOVA_SYSCALL_STATUS_UNSUPPORTED = 2,
    NOVA_SYSCALL_STATUS_DENIED = 3,
    NOVA_SYSCALL_STATUS_INVALID_ARGS = 4,
};

typedef struct NovaSyscallRequestV1 {
    uint32_t number;
    uint32_t flags;
    uint64_t args[NOVA_SYSCALL_ARG_COUNT];
} NovaSyscallRequestV1;

typedef struct NovaSyscallResultV1 {
    uint32_t status;
    uint32_t flags;
    uint64_t value0;
    uint64_t value1;
} NovaSyscallResultV1;

#ifdef __cplusplus
}
#endif

#endif /* NOVA_SYSCALL_V1_H */
