# Nova Image Digest v1

`NovaImageDigestV1` is the boot-time digest object referenced by `NovaBootInfoV1.kernel_image_hash_ptr`.

## Header

```c
struct nova_image_digest_v1 {
    uint64_t magic;
    uint32_t algorithm;
    uint32_t byte_len;
    uint8_t bytes[32];
};
```

## v1 Rules

- `magic` must equal `NOVA_IMAGE_DIGEST_V1_MAGIC`.
- `algorithm` currently supports only `NOVA_DIGEST_ALGORITHM_SHA256`.
- `byte_len` must equal `32`.
- the digest covers the staged kernel payload image bytes as loaded by stage0.

## Current Use

- stage0 computes the digest after loading `kernel.bin`
- stage0 stores the digest in persistent loader-owned memory
- `NovaBootInfoV1.kernel_image_hash_ptr` points at that digest object
