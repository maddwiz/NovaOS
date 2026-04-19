use core::fmt;
use core::mem::size_of;

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NovaBootInfoV1 {
    pub magic: u64,
    pub version: u32,
    pub flags: u32,

    pub firmware_vendor_ptr: u64,
    pub firmware_revision: u32,
    pub secure_boot_state: u8,
    pub boot_source: BootSource,
    pub current_el: u8,
    pub reserved0: u8,

    pub memory_map_ptr: u64,
    pub memory_map_entries: u32,
    pub memory_map_desc_size: u32,
    pub config_tables_ptr: u64,
    pub config_table_count: u32,
    pub reserved1: u32,

    pub acpi_rsdp_ptr: u64,
    pub dtb_ptr: u64,
    pub smbios_ptr: u64,

    pub framebuffer_base: u64,
    pub framebuffer_width: u32,
    pub framebuffer_height: u32,
    pub framebuffer_stride: u32,
    pub framebuffer_format: FramebufferFormat,

    pub init_capsule_ptr: u64,
    pub init_capsule_len: u64,
    pub kernel_image_hash_ptr: u64,
    pub loader_log_ptr: u64,
    pub verification_info_ptr: u64,
}

impl NovaBootInfoV1 {
    pub const MAGIC: u64 = 0x4E4F5641424F4F54;
    pub const VERSION: u32 = 1;

    pub const FLAG_HAS_ACPI_RSDP: u32 = 1 << 0;
    pub const FLAG_HAS_DTB: u32 = 1 << 1;
    pub const FLAG_HAS_SMBIOS: u32 = 1 << 2;
    pub const FLAG_HAS_FRAMEBUFFER: u32 = 1 << 3;
    pub const FLAG_HAS_LOADER_LOG: u32 = 1 << 4;
    pub const FLAG_HAS_KERNEL_IMAGE_DIGEST: u32 = 1 << 5;
    pub const FLAG_HAS_VERIFICATION_INFO: u32 = 1 << 6;

    pub const SECURE_BOOT_UNKNOWN: u8 = 0;
    pub const SECURE_BOOT_DISABLED: u8 = 1;
    pub const SECURE_BOOT_ENABLED: u8 = 2;

    pub const ZERO: Self = Self {
        magic: 0,
        version: 0,
        flags: 0,
        firmware_vendor_ptr: 0,
        firmware_revision: 0,
        secure_boot_state: 0,
        boot_source: BootSource::Unknown,
        current_el: 0,
        reserved0: 0,
        memory_map_ptr: 0,
        memory_map_entries: 0,
        memory_map_desc_size: 0,
        config_tables_ptr: 0,
        config_table_count: 0,
        reserved1: 0,
        acpi_rsdp_ptr: 0,
        dtb_ptr: 0,
        smbios_ptr: 0,
        framebuffer_base: 0,
        framebuffer_width: 0,
        framebuffer_height: 0,
        framebuffer_stride: 0,
        framebuffer_format: FramebufferFormat::Unknown,
        init_capsule_ptr: 0,
        init_capsule_len: 0,
        kernel_image_hash_ptr: 0,
        loader_log_ptr: 0,
        verification_info_ptr: 0,
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
        self.magic == Self::MAGIC && self.version == Self::VERSION
    }

    pub const fn has_flag(&self, flag: u32) -> bool {
        (self.flags & flag) != 0
    }

    pub fn set_flag(&mut self, flag: u32) {
        self.flags |= flag;
    }

    pub const fn framebuffer_present(&self) -> bool {
        self.framebuffer_base != 0 && self.framebuffer_width != 0 && self.framebuffer_height != 0
    }

    pub const fn memory_map_present(&self) -> bool {
        self.memory_map_ptr != 0 && self.memory_map_entries != 0 && self.memory_map_desc_size != 0
    }

    pub const fn memory_map_byte_len(&self) -> usize {
        (self.memory_map_entries as usize) * (self.memory_map_desc_size as usize)
    }

    pub const fn secure_boot_enabled(&self) -> bool {
        self.secure_boot_state == Self::SECURE_BOOT_ENABLED
    }

    pub const fn firmware(&self) -> FirmwareInfo {
        FirmwareInfo {
            secure_boot: self.secure_boot_enabled(),
            firmware_revision: self.firmware_revision,
            boot_source: self.boot_source,
        }
    }

    pub const fn memory(&self) -> MemoryInfo {
        MemoryInfo {
            kernel_window_base: self.memory_map_ptr,
            kernel_window_size: (self.memory_map_entries as u64)
                * (self.memory_map_desc_size as u64),
            user_window_base: self.config_tables_ptr,
            user_window_size: self.config_table_count as u64,
            usable_base: self.init_capsule_ptr,
            usable_limit: self.init_capsule_len,
            reserved_bytes: self.loader_log_ptr,
            region_count: self.memory_map_entries,
        }
    }

    pub const fn framebuffer(&self) -> FramebufferInfo {
        FramebufferInfo {
            base: self.framebuffer_base,
            width: self.framebuffer_width,
            height: self.framebuffer_height,
            stride: self.framebuffer_stride,
            format: self.framebuffer_format,
        }
    }

    pub const fn summary(&self) -> BootSummary {
        BootSummary {
            secure_boot: self.secure_boot_enabled(),
            framebuffer_present: self.framebuffer_present(),
            memory_map_entries: self.memory_map_entries,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BootSummary {
    pub secure_boot: bool,
    pub framebuffer_present: bool,
    pub memory_map_entries: u32,
}

impl BootSummary {
    pub const fn empty() -> Self {
        Self {
            secure_boot: false,
            framebuffer_present: false,
            memory_map_entries: 0,
        }
    }

    pub fn describe(self) -> &'static str {
        if self.framebuffer_present {
            "boot info parsed; framebuffer observed"
        } else if self.secure_boot {
            "boot info parsed; secure boot observed"
        } else if self.memory_map_entries != 0 {
            "boot info parsed; memory map observed"
        } else {
            "boot info parsed"
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BootSource {
    Unknown = 0,
    Usb = 1,
    BootOption = 2,
    Pxe = 3,
    InternalNvme = 4,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FramebufferFormat {
    Unknown = 0,
    Rgbx8888 = 1,
    Bgrx8888 = 2,
}

impl fmt::Display for BootSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.describe())
    }
}

const _: [(); 152] = [(); size_of::<NovaBootInfoV1>()];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FirmwareInfo {
    pub secure_boot: bool,
    pub firmware_revision: u32,
    pub boot_source: BootSource,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryInfo {
    pub kernel_window_base: u64,
    pub kernel_window_size: u64,
    pub user_window_base: u64,
    pub user_window_size: u64,
    pub usable_base: u64,
    pub usable_limit: u64,
    pub reserved_bytes: u64,
    pub region_count: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FramebufferInfo {
    pub base: u64,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: FramebufferFormat,
}

#[cfg(test)]
mod tests {
    use super::{BootSource, FramebufferFormat, NovaBootInfoV1};
    use core::mem::{offset_of, size_of};

    #[test]
    fn boot_info_layout_matches_c_header() {
        assert_eq!(size_of::<NovaBootInfoV1>(), 152);
        assert_eq!(offset_of!(NovaBootInfoV1, framebuffer_base), 88);
        assert_eq!(offset_of!(NovaBootInfoV1, verification_info_ptr), 144);
    }

    #[test]
    fn boot_info_new_is_valid() {
        let info = NovaBootInfoV1::new();
        assert!(info.is_valid());
    }

    #[test]
    fn memory_map_helpers_report_expected_state() {
        let mut info = NovaBootInfoV1::new();
        assert!(!info.memory_map_present());
        assert_eq!(info.memory_map_byte_len(), 0);

        info.memory_map_ptr = 0x1000;
        info.memory_map_entries = 4;
        info.memory_map_desc_size = 48;

        assert!(info.memory_map_present());
        assert_eq!(info.memory_map_byte_len(), 192);
    }

    #[test]
    fn enums_match_public_abi_values() {
        assert_eq!(BootSource::Usb as u8, 1);
        assert_eq!(BootSource::BootOption as u8, 2);
        assert_eq!(BootSource::Pxe as u8, 3);
        assert_eq!(BootSource::InternalNvme as u8, 4);
        assert_eq!(FramebufferFormat::Rgbx8888 as u32, 1);
        assert_eq!(FramebufferFormat::Bgrx8888 as u32, 2);
    }
}
