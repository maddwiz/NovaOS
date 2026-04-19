# Nova Payload v1

Nova Payload v1 is the typed wrapper format for early freestanding NovaOS images such as:

- `stage1.bin`
- `kernel.bin`
- embedded bootstrap service images such as `initd`

It exists so stage0 and stage1 stop treating arbitrary byte blobs as executable payloads.

## Header

```c
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
```

## v1 Rules

- `magic` must equal `NOVA_PAYLOAD_V1_MAGIC`.
- `version` must equal `1`.
- `header_size` must equal `sizeof(struct nova_payload_header_v1)`.
- `image_size` must equal the full wrapped file length.
- `load_mode` currently supports only `NOVA_PAYLOAD_LOAD_MODE_FLAT_BINARY`.
- `load_offset` must equal `header_size`.
- `load_size` must equal `image_size - load_offset`.
- `entry_offset` must point inside the declared load window.
- `kind` must match the expected payload role.
- `entry_abi` must match the expected entry contract for the payload role.
- `body_digest_algorithm` currently supports only SHA-256.
- `body_digest_len` must equal `32`.
- `body_digest` must match the declared load bytes.

## Kinds

- `NOVA_PAYLOAD_KIND_STAGE1`
- `NOVA_PAYLOAD_KIND_KERNEL`
- `NOVA_PAYLOAD_KIND_SERVICE`

## Current Use

- `boot/mkimage` wraps raw freestanding binaries into payload images.
- `stage1.bin` currently uses the flat-binary load mode with the `STAGE1_PLAN` entry ABI.
- `kernel.bin` currently uses the flat-binary load mode with the `BOOTINFO_V2_SIDECAR` entry ABI.
- the embedded bootstrap service payload inside `init.capsule` now uses the `SERVICE` kind with the `BOOTSTRAP_TASK_V1` entry ABI.
- under the current Arm64 raw-entry path, `BOOTSTRAP_TASK_V1` now means the kernel enters the payload on a dedicated bootstrap-task stack, passes a typed bootstrap-task context pointer in `x0`, exposes a transitional same-EL bootstrap kernel-call gate through that context, and clears the rest of the early argument registers before the first instruction runs.
- stage0 validates the `stage1.bin` header, executable contract, and body digest before transferring control.
- stage1 validates the `kernel.bin` header, executable contract, and body digest before computing the kernel entry point.
