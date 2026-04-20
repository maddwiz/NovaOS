# NovaBootInfoV2 Draft

`NovaBootInfoV2` is the portability-oriented successor to the current Spark-era BootInfo v1 contract.

It is now wired into the live loader as a transitional sidecar that stage1 carries into the raw Arm64 kernel entry while the main bring-up contract still remains `NovaBootInfoV1`.

## Goals

- describe platform family and CPU architecture explicitly
- distinguish memory-topology classes such as UMA, discrete VRAM, NVLink-style peer pools, and MIG-like partitions
- expose accelerator seeds as boot facts instead of driver APIs
- preserve framebuffer, display-path, storage-seed, and network-seed discovery results
- carry loader/kernel hash pointers and optional observatory hashes forward into later boot stages
- snapshot an optional embedded bootstrap-task payload descriptor so later stages can keep `initd` image facts without reparsing `init.capsule`

## Mandatory portability additions

Compared with v1, the draft adds:

- `cpu_arch`
- `platform_class`
- `memory_topology_class`
- `vendor_tables_ptr` and `vendor_table_count`
- `display_path`
- storage seed pointer/count
- network seed pointer/count
- accelerator seed pointer/count
- `loader_log_len`
- `loader_image_hash_ptr`
- `boot_counter`
- `observatory_hash_ptr`
- `bootstrap_payload`
- `bootstrap_user_window`

`bootstrap_user_window` is empty by default. When present, it describes the initial 4 KiB page-aligned EL0 bootstrap user window and stack reservation that the kernel may use to rebase the embedded bootstrap payload instead of treating firmware config-table fields as a user address-space contract. Its `flags` field is reserved and must be zero in the current draft.

## Accelerator seed doctrine

`NovaAccelSeedV1` carries facts only:

- vendor and class identity if known
- transport type: integrated, platform, PCI, or fabric
- topology hint: UMA, discrete, partitionable, or linked
- already-discovered MMIO windows
- an interrupt hint if the firmware exposes one
- an optional pointer to a raw platform or firmware table blob

It is not a stable userspace driver ABI and it must not contain guessed register maps.
