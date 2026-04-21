use core::mem::size_of;

use crate::{BootSource, FramebufferFormat};
use nova_fabric::{CpuArchitecture, MemoryTopologyClass, PlatformClass};

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NovaFramebufferDescriptorV1 {
    pub base: u64,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: FramebufferFormat,
}

impl NovaFramebufferDescriptorV1 {
    pub const fn empty() -> Self {
        Self {
            base: 0,
            width: 0,
            height: 0,
            stride: 0,
            format: FramebufferFormat::Unknown,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NovaDisplayPathDescriptorV1 {
    pub device_path_ptr: u64,
    pub device_path_len: u32,
    pub flags: u32,
}

impl NovaDisplayPathDescriptorV1 {
    pub const fn empty() -> Self {
        Self {
            device_path_ptr: 0,
            device_path_len: 0,
            flags: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NovaStorageSeedV1 {
    pub device_path_ptr: u64,
    pub device_path_len: u32,
    pub flags: u32,
}

impl NovaStorageSeedV1 {
    pub const fn empty() -> Self {
        Self {
            device_path_ptr: 0,
            device_path_len: 0,
            flags: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NovaNetworkSeedV1 {
    pub device_path_ptr: u64,
    pub device_path_len: u32,
    pub flags: u32,
}

impl NovaNetworkSeedV1 {
    pub const fn empty() -> Self {
        Self {
            device_path_ptr: 0,
            device_path_len: 0,
            flags: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NovaBootstrapPayloadDescriptorV1 {
    pub image_ptr: u64,
    pub image_len: u64,
    pub load_base: u64,
    pub load_size: u64,
    pub entry_point: u64,
}

impl NovaBootstrapPayloadDescriptorV1 {
    pub const fn empty() -> Self {
        Self {
            image_ptr: 0,
            image_len: 0,
            load_base: 0,
            load_size: 0,
            entry_point: 0,
        }
    }

    pub const fn is_empty(&self) -> bool {
        self.image_ptr == 0
            && self.image_len == 0
            && self.load_base == 0
            && self.load_size == 0
            && self.entry_point == 0
    }

    pub const fn is_valid(&self) -> bool {
        if self.is_empty() {
            return true;
        }

        if self.image_ptr == 0
            || self.image_len == 0
            || self.load_base == 0
            || self.load_size == 0
            || self.entry_point == 0
        {
            return false;
        }

        let Some(image_limit) = self.image_ptr.checked_add(self.image_len) else {
            return false;
        };
        let Some(load_limit) = self.load_base.checked_add(self.load_size) else {
            return false;
        };

        self.load_base >= self.image_ptr
            && self.load_base < image_limit
            && load_limit <= image_limit
            && self.entry_point >= self.load_base
            && self.entry_point < load_limit
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NovaBootstrapUserWindowDescriptorV1 {
    pub base: u64,
    pub len: u64,
    pub stack_size: u64,
    pub page_size: u32,
    pub flags: u32,
}

impl NovaBootstrapUserWindowDescriptorV1 {
    pub const PAGE_SIZE_4K: u32 = 4096;

    pub const fn empty() -> Self {
        Self {
            base: 0,
            len: 0,
            stack_size: 0,
            page_size: 0,
            flags: 0,
        }
    }

    pub const fn is_empty(&self) -> bool {
        self.base == 0
            && self.len == 0
            && self.stack_size == 0
            && self.page_size == 0
            && self.flags == 0
    }

    pub const fn is_valid(&self) -> bool {
        if self.is_empty() {
            return true;
        }

        if self.base == 0
            || self.len == 0
            || self.stack_size == 0
            || self.page_size != Self::PAGE_SIZE_4K
            || self.flags != 0
        {
            return false;
        }

        let page_size = Self::PAGE_SIZE_4K as u64;
        self.base % page_size == 0
            && self.len % page_size == 0
            && self.stack_size % page_size == 0
            && self.stack_size <= self.len
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NovaBootstrapFrameArenaDescriptorV1 {
    pub base: u64,
    pub len: u64,
    pub page_size: u32,
    pub flags: u32,
}

impl NovaBootstrapFrameArenaDescriptorV1 {
    pub const PAGE_SIZE_4K: u32 = 4096;

    pub const fn empty() -> Self {
        Self {
            base: 0,
            len: 0,
            page_size: 0,
            flags: 0,
        }
    }

    pub const fn is_empty(&self) -> bool {
        self.base == 0 && self.len == 0 && self.page_size == 0 && self.flags == 0
    }

    pub const fn is_valid(&self) -> bool {
        if self.is_empty() {
            return true;
        }

        if self.base == 0
            || self.len == 0
            || self.page_size != Self::PAGE_SIZE_4K
            || self.flags != 0
        {
            return false;
        }

        let page_size = Self::PAGE_SIZE_4K as u64;
        self.base % page_size == 0 && self.len % page_size == 0
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NovaBootInfoV2 {
    pub magic: u64,
    pub version: u32,
    pub flags: u32,

    pub cpu_arch: CpuArchitecture,
    pub platform_class: PlatformClass,
    pub memory_topology_class: MemoryTopologyClass,
    pub secure_boot_state: u8,
    pub boot_source: BootSource,
    pub current_el: u8,
    pub reserved0: u8,
    pub reserved1: u16,

    pub firmware_vendor_ptr: u64,
    pub firmware_revision: u32,
    pub reserved2: u32,

    pub memory_map_ptr: u64,
    pub memory_map_entries: u32,
    pub memory_map_desc_size: u32,
    pub config_tables_ptr: u64,
    pub config_table_count: u32,
    pub reserved3: u32,

    pub acpi_rsdp_ptr: u64,
    pub dtb_ptr: u64,
    pub smbios_ptr: u64,
    pub vendor_tables_ptr: u64,
    pub vendor_table_count: u32,
    pub reserved4: u32,

    pub framebuffer: NovaFramebufferDescriptorV1,
    pub display_path: NovaDisplayPathDescriptorV1,

    pub storage_seeds_ptr: u64,
    pub storage_seed_count: u32,
    pub reserved5: u32,
    pub network_seeds_ptr: u64,
    pub network_seed_count: u32,
    pub reserved6: u32,
    pub accel_seeds_ptr: u64,
    pub accel_seed_count: u32,
    pub reserved7: u32,

    pub init_capsule_ptr: u64,
    pub init_capsule_len: u64,
    pub loader_log_ptr: u64,
    pub loader_log_len: u64,
    pub kernel_image_hash_ptr: u64,
    pub loader_image_hash_ptr: u64,
    pub boot_counter: u64,
    pub observatory_hash_ptr: u64,
    pub bootstrap_payload: NovaBootstrapPayloadDescriptorV1,
    pub bootstrap_user_window: NovaBootstrapUserWindowDescriptorV1,
    pub bootstrap_frame_arena: NovaBootstrapFrameArenaDescriptorV1,
}

impl NovaBootInfoV2 {
    pub const MAGIC: u64 = 0x3242_4F4F_5441_564E;
    pub const VERSION: u32 = 2;

    pub const ZERO: Self = Self {
        magic: 0,
        version: 0,
        flags: 0,
        cpu_arch: CpuArchitecture::Unknown,
        platform_class: PlatformClass::Unknown,
        memory_topology_class: MemoryTopologyClass::Unknown,
        secure_boot_state: 0,
        boot_source: BootSource::Unknown,
        current_el: 0,
        reserved0: 0,
        reserved1: 0,
        firmware_vendor_ptr: 0,
        firmware_revision: 0,
        reserved2: 0,
        memory_map_ptr: 0,
        memory_map_entries: 0,
        memory_map_desc_size: 0,
        config_tables_ptr: 0,
        config_table_count: 0,
        reserved3: 0,
        acpi_rsdp_ptr: 0,
        dtb_ptr: 0,
        smbios_ptr: 0,
        vendor_tables_ptr: 0,
        vendor_table_count: 0,
        reserved4: 0,
        framebuffer: NovaFramebufferDescriptorV1::empty(),
        display_path: NovaDisplayPathDescriptorV1::empty(),
        storage_seeds_ptr: 0,
        storage_seed_count: 0,
        reserved5: 0,
        network_seeds_ptr: 0,
        network_seed_count: 0,
        reserved6: 0,
        accel_seeds_ptr: 0,
        accel_seed_count: 0,
        reserved7: 0,
        init_capsule_ptr: 0,
        init_capsule_len: 0,
        loader_log_ptr: 0,
        loader_log_len: 0,
        kernel_image_hash_ptr: 0,
        loader_image_hash_ptr: 0,
        boot_counter: 0,
        observatory_hash_ptr: 0,
        bootstrap_payload: NovaBootstrapPayloadDescriptorV1::empty(),
        bootstrap_user_window: NovaBootstrapUserWindowDescriptorV1::empty(),
        bootstrap_frame_arena: NovaBootstrapFrameArenaDescriptorV1::empty(),
    };

    pub const fn new() -> Self {
        Self {
            magic: Self::MAGIC,
            version: Self::VERSION,
            ..Self::ZERO
        }
    }

    pub const fn empty() -> Self {
        Self::ZERO
    }

    pub const fn is_valid(&self) -> bool {
        self.magic == Self::MAGIC
            && self.version == Self::VERSION
            && self.bootstrap_payload.is_valid()
            && self.bootstrap_user_window.is_valid()
            && self.bootstrap_frame_arena.is_valid()
    }

    pub const fn memory_map_present(&self) -> bool {
        self.memory_map_ptr != 0 && self.memory_map_entries != 0 && self.memory_map_desc_size != 0
    }

    pub const fn framebuffer_present(&self) -> bool {
        self.framebuffer.base != 0 && self.framebuffer.width != 0 && self.framebuffer.height != 0
    }
}

const _: [(); 40] = [(); size_of::<NovaBootstrapPayloadDescriptorV1>()];
const _: [(); 32] = [(); size_of::<NovaBootstrapUserWindowDescriptorV1>()];
const _: [(); 24] = [(); size_of::<NovaBootstrapFrameArenaDescriptorV1>()];
const _: [(); 368] = [(); size_of::<NovaBootInfoV2>()];

#[cfg(test)]
mod tests {
    use super::{
        NovaBootInfoV2, NovaBootstrapFrameArenaDescriptorV1, NovaBootstrapPayloadDescriptorV1,
        NovaBootstrapUserWindowDescriptorV1, NovaDisplayPathDescriptorV1,
        NovaFramebufferDescriptorV1, NovaNetworkSeedV1, NovaStorageSeedV1,
    };
    use crate::{BootSource, FramebufferFormat};
    use core::mem::{offset_of, size_of};
    use nova_fabric::{CpuArchitecture, MemoryTopologyClass, PlatformClass};

    #[test]
    fn boot_info_v2_layout_matches_c_header() {
        assert_eq!(size_of::<NovaFramebufferDescriptorV1>(), 24);
        assert_eq!(size_of::<NovaDisplayPathDescriptorV1>(), 16);
        assert_eq!(size_of::<NovaStorageSeedV1>(), 16);
        assert_eq!(size_of::<NovaNetworkSeedV1>(), 16);
        assert_eq!(size_of::<NovaBootstrapPayloadDescriptorV1>(), 40);
        assert_eq!(size_of::<NovaBootstrapUserWindowDescriptorV1>(), 32);
        assert_eq!(size_of::<NovaBootstrapFrameArenaDescriptorV1>(), 24);
        assert_eq!(size_of::<NovaBootInfoV2>(), 368);

        assert_eq!(offset_of!(NovaBootInfoV2, cpu_arch), 16);
        assert_eq!(offset_of!(NovaBootInfoV2, firmware_vendor_ptr), 32);
        assert_eq!(offset_of!(NovaBootInfoV2, framebuffer), 120);
        assert_eq!(offset_of!(NovaBootInfoV2, accel_seeds_ptr), 192);
        assert_eq!(offset_of!(NovaBootInfoV2, init_capsule_ptr), 208);
        assert_eq!(offset_of!(NovaBootInfoV2, bootstrap_payload), 272);
        assert_eq!(offset_of!(NovaBootInfoV2, bootstrap_user_window), 312);
        assert_eq!(offset_of!(NovaBootInfoV2, bootstrap_frame_arena), 344);
    }

    #[test]
    fn boot_info_v2_defaults_match_portable_contract() {
        let info = NovaBootInfoV2::new();

        assert!(info.is_valid());
        assert_eq!(info.cpu_arch, CpuArchitecture::Unknown);
        assert_eq!(info.platform_class, PlatformClass::Unknown);
        assert_eq!(info.memory_topology_class, MemoryTopologyClass::Unknown);
        assert_eq!(info.boot_source, BootSource::Unknown);
        assert_eq!(info.framebuffer.format, FramebufferFormat::Unknown);
        assert!(info.bootstrap_payload.is_empty());
        assert!(info.bootstrap_user_window.is_empty());
        assert!(info.bootstrap_frame_arena.is_empty());
        assert!(!info.memory_map_present());
        assert!(!info.framebuffer_present());
    }

    #[test]
    fn bootstrap_payload_descriptor_requires_coherent_ranges() {
        let empty = NovaBootstrapPayloadDescriptorV1::empty();
        assert!(empty.is_valid());

        let partial = NovaBootstrapPayloadDescriptorV1 {
            image_ptr: 0x1000,
            ..NovaBootstrapPayloadDescriptorV1::empty()
        };
        assert!(!partial.is_valid());

        let valid = NovaBootstrapPayloadDescriptorV1 {
            image_ptr: 0x1000,
            image_len: 0x80,
            load_base: 0x1030,
            load_size: 0x40,
            entry_point: 0x1030,
        };
        assert!(valid.is_valid());

        let invalid_entry = NovaBootstrapPayloadDescriptorV1 {
            entry_point: 0x1080,
            ..valid
        };
        assert!(!invalid_entry.is_valid());
    }

    #[test]
    fn bootstrap_user_window_descriptor_requires_page_aligned_window_and_stack() {
        let empty = NovaBootstrapUserWindowDescriptorV1::empty();
        assert!(empty.is_valid());

        let valid = NovaBootstrapUserWindowDescriptorV1 {
            base: 0x4000_0000,
            len: 0x20_000,
            stack_size: 0x8000,
            page_size: NovaBootstrapUserWindowDescriptorV1::PAGE_SIZE_4K,
            flags: 0,
        };
        assert!(valid.is_valid());

        assert!(
            !NovaBootstrapUserWindowDescriptorV1 {
                base: 0x4000_0001,
                ..valid
            }
            .is_valid()
        );
        assert!(
            !NovaBootstrapUserWindowDescriptorV1 {
                stack_size: 0x40_000,
                ..valid
            }
            .is_valid()
        );
        assert!(
            !NovaBootstrapUserWindowDescriptorV1 {
                page_size: 16 * 1024,
                ..valid
            }
            .is_valid()
        );
        assert!(!NovaBootstrapUserWindowDescriptorV1 { flags: 1, ..valid }.is_valid());
    }

    #[test]
    fn bootstrap_frame_arena_descriptor_requires_page_aligned_arena() {
        let empty = NovaBootstrapFrameArenaDescriptorV1::empty();
        assert!(empty.is_valid());

        let valid = NovaBootstrapFrameArenaDescriptorV1 {
            base: 0x9000_0000,
            len: 0x20_000,
            page_size: NovaBootstrapFrameArenaDescriptorV1::PAGE_SIZE_4K,
            flags: 0,
        };
        assert!(valid.is_valid());

        assert!(
            !NovaBootstrapFrameArenaDescriptorV1 {
                base: 0x9000_0001,
                ..valid
            }
            .is_valid()
        );
        assert!(
            !NovaBootstrapFrameArenaDescriptorV1 {
                len: 0x20_001,
                ..valid
            }
            .is_valid()
        );
        assert!(
            !NovaBootstrapFrameArenaDescriptorV1 {
                page_size: 16 * 1024,
                ..valid
            }
            .is_valid()
        );
        assert!(!NovaBootstrapFrameArenaDescriptorV1 { flags: 1, ..valid }.is_valid());
    }
}
