#![allow(dead_code)]

use core::fmt;
use core::fmt::Write as _;

#[cfg(target_os = "uefi")]
extern crate alloc;

#[cfg(target_os = "uefi")]
use alloc::string::{String, ToString};
#[cfg(target_os = "uefi")]
use alloc::vec::Vec;
use nova_fabric::{AccelSeedV1, AccelTopologyHint, AccelTransport, MemoryTopologyClass};
#[cfg(not(target_os = "uefi"))]
use std::string::String;
#[cfg(not(target_os = "uefi"))]
use std::vec::Vec;
#[cfg(target_os = "uefi")]
use uefi::boot::{self, MemoryType};
#[cfg(target_os = "uefi")]
use uefi::fs::FileSystem;
#[cfg(target_os = "uefi")]
use uefi::mem::memory_map::MemoryMap;
#[cfg(target_os = "uefi")]
use uefi::proto::console::gop::{GraphicsOutput, PixelFormat};
#[cfg(target_os = "uefi")]
use uefi::proto::device_path::DevicePath;
#[cfg(target_os = "uefi")]
use uefi::proto::device_path::text::{AllowShortcuts, DisplayOnly};
#[cfg(target_os = "uefi")]
use uefi::proto::loaded_image::LoadedImage;
#[cfg(target_os = "uefi")]
use uefi::proto::media::block::BlockIO;
#[cfg(target_os = "uefi")]
use uefi::proto::media::fs::SimpleFileSystem;
#[cfg(target_os = "uefi")]
use uefi::proto::network::snp::SimpleNetwork;
#[cfg(target_os = "uefi")]
use uefi::runtime::{self, VariableVendor};
#[cfg(target_os = "uefi")]
use uefi::system;
#[cfg(target_os = "uefi")]
use uefi::table::cfg;
#[cfg(target_os = "uefi")]
use uefi::{boot as uefi_boot, cstr16, guid};

#[cfg(target_os = "uefi")]
const DEVICE_TREE_GUID: uefi::Guid = guid!("b1b621d5-f19c-41a5-830b-d9152c69aae0");
pub const STRUCTURED_REPORT_PATH: &str = r"\nova\observatory\spark-observe-report.txt";
pub const STRUCTURED_REPORT_FALLBACK_PATH: &str = r"\EFI\BOOT\spark-observe-report.txt";
const DISPLAY_FLAG_GOP_HANDLE: u32 = 1 << 0;
const STORAGE_FLAG_FILESYSTEM_HANDLE: u32 = 1 << 0;
const STORAGE_FLAG_BLOCK_HANDLE: u32 = 1 << 1;
const NETWORK_FLAG_SIMPLE_NETWORK_HANDLE: u32 = 1 << 0;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BootObservationFlags {
    pub secure_boot_known: bool,
    pub acpi_rsdp_present: bool,
    pub dtb_present: bool,
    pub smbios_present: bool,
    pub framebuffer_present: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BootTablePresence {
    pub acpi_rsdp: Option<u64>,
    pub dtb: Option<u64>,
    pub smbios: Option<u64>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FramebufferObservation {
    pub base: u64,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub pixel_format: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SeedPathObservation {
    pub device_path: String,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccelSeedObservation {
    pub seed: AccelSeedV1,
    pub source: &'static str,
    pub raw_device_path: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BootObservation {
    pub firmware_vendor: Option<String>,
    pub firmware_revision: u32,
    pub secure_boot_enabled: Option<bool>,
    pub setup_mode: Option<u8>,
    pub flags: BootObservationFlags,
    pub table_presence: BootTablePresence,
    pub framebuffer: Option<FramebufferObservation>,
    pub config_table_count: usize,
    pub memory_map_entries: usize,
    pub memory_map_desc_size: usize,
    pub conventional_pages: u64,
    pub loaded_image_path: Option<String>,
    pub loaded_image_path_known: bool,
    pub storage_filesystem_handles: usize,
    pub storage_block_handles: usize,
    pub network_handles: usize,
    pub display_paths: Vec<SeedPathObservation>,
    pub storage_seeds: Vec<SeedPathObservation>,
    pub network_seeds: Vec<SeedPathObservation>,
    pub accel_seed_drafts: Vec<AccelSeedObservation>,
}

impl Default for BootObservation {
    fn default() -> Self {
        Self {
            firmware_vendor: None,
            firmware_revision: 0,
            secure_boot_enabled: None,
            setup_mode: None,
            flags: BootObservationFlags::default(),
            table_presence: BootTablePresence::default(),
            framebuffer: None,
            config_table_count: 0,
            memory_map_entries: 0,
            memory_map_desc_size: 0,
            conventional_pages: 0,
            loaded_image_path: None,
            loaded_image_path_known: false,
            storage_filesystem_handles: 0,
            storage_block_handles: 0,
            network_handles: 0,
            display_paths: Vec::new(),
            storage_seeds: Vec::new(),
            network_seeds: Vec::new(),
            accel_seed_drafts: Vec::new(),
        }
    }
}

impl BootObservation {
    pub fn collect() -> Self {
        #[cfg(target_os = "uefi")]
        {
            Self::collect_uefi()
        }

        #[cfg(not(target_os = "uefi"))]
        {
            Self::default()
        }
    }

    #[cfg(target_os = "uefi")]
    fn collect_uefi() -> Self {
        let mut observation = Self {
            firmware_vendor: Some(system::firmware_vendor().to_string()),
            firmware_revision: system::firmware_revision(),
            ..Self::default()
        };

        let (config_table_count, table_presence, table_flags) =
            system::with_config_table(|tables| {
                let mut flags = BootObservationFlags::default();
                let mut presence = BootTablePresence::default();

                for table in tables {
                    let address = table.address as u64;

                    if table.guid == cfg::ACPI_GUID || table.guid == cfg::ACPI2_GUID {
                        flags.acpi_rsdp_present = true;
                        presence.acpi_rsdp = Some(address);
                    }

                    if table.guid == cfg::SMBIOS_GUID || table.guid == cfg::SMBIOS3_GUID {
                        flags.smbios_present = true;
                        presence.smbios = Some(address);
                    }

                    if table.guid == DEVICE_TREE_GUID {
                        flags.dtb_present = true;
                        presence.dtb = Some(address);
                    }
                }

                (tables.len(), presence, flags)
            });

        observation.config_table_count = config_table_count;
        observation.table_presence = table_presence;
        observation.flags.acpi_rsdp_present = table_flags.acpi_rsdp_present;
        observation.flags.dtb_present = table_flags.dtb_present;
        observation.flags.smbios_present = table_flags.smbios_present;

        observation.secure_boot_enabled =
            read_u8_variable(cstr16!("SecureBoot")).map(|value| value != 0);
        observation.setup_mode = read_u8_variable(cstr16!("SetupMode"));
        observation.flags.secure_boot_known = observation.secure_boot_enabled.is_some();

        if let Ok(memory_map) = boot::memory_map(MemoryType::LOADER_DATA) {
            observation.memory_map_entries = memory_map.entries().count();
            observation.memory_map_desc_size = memory_map.meta().desc_size;
            observation.conventional_pages = memory_map
                .entries()
                .filter(|entry| entry.ty == MemoryType::CONVENTIONAL)
                .map(|entry| entry.page_count)
                .sum();
        }

        observation.loaded_image_path = Self::loaded_image_path_text();
        observation.loaded_image_path_known = observation.loaded_image_path.is_some();

        if let Ok(handle) = uefi_boot::get_handle_for_protocol::<GraphicsOutput>() {
            if let Ok(mut gop) = uefi_boot::open_protocol_exclusive::<GraphicsOutput>(handle) {
                let mode = gop.current_mode_info();
                let (width, height) = mode.resolution();
                let stride = mode.stride();
                let base = gop.frame_buffer().as_mut_ptr() as u64;

                observation.flags.framebuffer_present = true;
                observation.framebuffer = Some(FramebufferObservation {
                    base,
                    width: width as u32,
                    height: height as u32,
                    stride: stride as u32,
                    pixel_format: pixel_format_name(mode.pixel_format()),
                });
            }

            if let Some(device_path) = handle_device_path_text(handle) {
                merge_seed_path(
                    &mut observation.display_paths,
                    device_path,
                    DISPLAY_FLAG_GOP_HANDLE,
                );
            }
        }

        if let Ok(handles) = boot::find_handles::<SimpleFileSystem>() {
            observation.storage_filesystem_handles = handles.len();
            for handle in handles {
                if let Some(device_path) = handle_device_path_text(handle) {
                    merge_seed_path(
                        &mut observation.storage_seeds,
                        device_path,
                        STORAGE_FLAG_FILESYSTEM_HANDLE,
                    );
                }
            }
        }

        if let Ok(handles) = boot::find_handles::<BlockIO>() {
            observation.storage_block_handles = handles.len();
            for handle in handles {
                if let Some(device_path) = handle_device_path_text(handle) {
                    merge_seed_path(
                        &mut observation.storage_seeds,
                        device_path,
                        STORAGE_FLAG_BLOCK_HANDLE,
                    );
                }
            }
        }

        if let Ok(handles) = boot::find_handles::<SimpleNetwork>() {
            observation.network_handles = handles.len();
            for handle in handles {
                if let Some(device_path) = handle_device_path_text(handle) {
                    merge_seed_path(
                        &mut observation.network_seeds,
                        device_path,
                        NETWORK_FLAG_SIMPLE_NETWORK_HANDLE,
                    );
                }
            }
        }

        observation.accel_seed_drafts = build_accel_seed_drafts(&observation);

        observation
    }

    #[cfg(target_os = "uefi")]
    pub fn loaded_image_path_text() -> Option<String> {
        let loaded_image =
            uefi_boot::open_protocol_exclusive::<LoadedImage>(uefi_boot::image_handle()).ok()?;
        let device_path = loaded_image.file_path()?;
        let text = device_path
            .to_string(DisplayOnly(false), AllowShortcuts(true))
            .ok()?;
        Some(text.to_string())
    }

    pub fn structured_report(&self) -> String {
        let mut report = String::new();

        let _ = writeln!(report, "report_kind=spark_observatory_v2_seed_report");
        let _ = writeln!(report, "report_version=1");
        if let Some(vendor) = self.firmware_vendor.as_deref() {
            let _ = writeln!(report, "firmware_vendor={vendor}");
        } else {
            let _ = writeln!(report, "firmware_vendor=unknown");
        }
        let _ = writeln!(report, "firmware_revision={}", self.firmware_revision);
        let _ = writeln!(report, "config_tables={}", self.config_table_count);
        let _ = writeln!(report, "memory_map_entries={}", self.memory_map_entries);
        let _ = writeln!(report, "memory_map_desc_size={}", self.memory_map_desc_size);
        let _ = writeln!(report, "conventional_pages={}", self.conventional_pages);
        if let Some(path) = self.loaded_image_path.as_deref() {
            let _ = writeln!(report, "loaded_image_path={path}");
        } else {
            let _ = writeln!(report, "loaded_image_path=unknown");
        }
        let _ = writeln!(
            report,
            "loaded_image_path_known={}",
            self.loaded_image_path_known
        );
        let _ = writeln!(
            report,
            "storage_filesystem_handles={}",
            self.storage_filesystem_handles
        );
        let _ = writeln!(
            report,
            "storage_block_handles={}",
            self.storage_block_handles
        );
        let _ = writeln!(report, "network_handles={}", self.network_handles);
        if let Some(enabled) = self.secure_boot_enabled {
            let _ = writeln!(report, "secure_boot_enabled={enabled}");
        } else {
            let _ = writeln!(report, "secure_boot_enabled=unknown");
        }
        if let Some(mode) = self.setup_mode {
            let _ = writeln!(report, "setup_mode={mode}");
        } else {
            let _ = writeln!(report, "setup_mode=unknown");
        }
        if let Some(ptr) = self.table_presence.acpi_rsdp {
            let _ = writeln!(report, "acpi_rsdp={ptr:#x}");
        } else {
            let _ = writeln!(report, "acpi_rsdp=absent");
        }
        if let Some(ptr) = self.table_presence.dtb {
            let _ = writeln!(report, "dtb={ptr:#x}");
        } else {
            let _ = writeln!(report, "dtb=absent");
        }
        if let Some(ptr) = self.table_presence.smbios {
            let _ = writeln!(report, "smbios={ptr:#x}");
        } else {
            let _ = writeln!(report, "smbios=absent");
        }
        if let Some(fb) = self.framebuffer {
            let _ = writeln!(report, "framebuffer.base={:#x}", fb.base);
            let _ = writeln!(report, "framebuffer.width={}", fb.width);
            let _ = writeln!(report, "framebuffer.height={}", fb.height);
            let _ = writeln!(report, "framebuffer.stride={}", fb.stride);
            let _ = writeln!(report, "framebuffer.format={}", fb.pixel_format);
        } else {
            let _ = writeln!(report, "framebuffer=absent");
        }

        emit_seed_paths(&mut report, "display_seed", &self.display_paths);
        emit_seed_paths(&mut report, "storage_seed", &self.storage_seeds);
        emit_seed_paths(&mut report, "network_seed", &self.network_seeds);
        emit_accel_seed_drafts(&mut report, &self.accel_seed_drafts);

        report
    }

    #[cfg(target_os = "uefi")]
    pub fn persist_report(&self) -> Option<String> {
        let report = self.structured_report();
        let fs_proto = boot::get_image_file_system(boot::image_handle()).ok()?;
        let mut fs = FileSystem::new(fs_proto);

        if fs.create_dir_all(cstr16!(r"\nova\observatory")).is_ok()
            && fs
                .write(
                    cstr16!(r"\nova\observatory\spark-observe-report.txt"),
                    report.as_bytes(),
                )
                .is_ok()
        {
            return Some(String::from(STRUCTURED_REPORT_PATH));
        }

        if fs
            .write(
                cstr16!(r"\EFI\BOOT\spark-observe-report.txt"),
                report.as_bytes(),
            )
            .is_ok()
        {
            return Some(String::from(STRUCTURED_REPORT_FALLBACK_PATH));
        }

        None
    }

    pub fn summary_lines(&self) -> impl fmt::Display + '_ {
        struct Summary<'a>(&'a BootObservation);

        impl fmt::Display for Summary<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                if let Some(vendor) = self.0.firmware_vendor.as_deref() {
                    writeln!(f, "firmware_vendor={vendor}")?;
                } else {
                    writeln!(f, "firmware_vendor=unknown")?;
                }
                writeln!(f, "firmware_revision={}", self.0.firmware_revision)?;
                writeln!(f, "config_tables={}", self.0.config_table_count)?;
                writeln!(f, "memory_map_entries={}", self.0.memory_map_entries)?;
                writeln!(f, "memory_map_desc_size={}", self.0.memory_map_desc_size)?;
                writeln!(f, "conventional_pages={}", self.0.conventional_pages)?;
                writeln!(
                    f,
                    "loaded_image_path_known={}",
                    self.0.loaded_image_path_known
                )?;
                if let Some(path) = self.0.loaded_image_path.as_deref() {
                    writeln!(f, "loaded_image_path={path}")?;
                } else {
                    writeln!(f, "loaded_image_path=unknown")?;
                }
                writeln!(
                    f,
                    "storage_filesystem_handles={}",
                    self.0.storage_filesystem_handles
                )?;
                writeln!(f, "storage_block_handles={}", self.0.storage_block_handles)?;
                writeln!(f, "network_handles={}", self.0.network_handles)?;
                writeln!(f, "display_seed_count={}", self.0.display_paths.len())?;
                writeln!(f, "storage_seed_count={}", self.0.storage_seeds.len())?;
                writeln!(f, "network_seed_count={}", self.0.network_seeds.len())?;
                writeln!(
                    f,
                    "accel_seed_draft_count={}",
                    self.0.accel_seed_drafts.len()
                )?;
                if let Some(enabled) = self.0.secure_boot_enabled {
                    writeln!(f, "secure_boot_enabled={enabled}")?;
                } else {
                    writeln!(f, "secure_boot_enabled=unknown")?;
                }
                if let Some(mode) = self.0.setup_mode {
                    writeln!(f, "setup_mode={mode}")?;
                } else {
                    writeln!(f, "setup_mode=unknown")?;
                }
                if let Some(ptr) = self.0.table_presence.acpi_rsdp {
                    writeln!(f, "acpi_rsdp={ptr:#x}")?;
                } else {
                    writeln!(f, "acpi_rsdp=absent")?;
                }
                if let Some(ptr) = self.0.table_presence.dtb {
                    writeln!(f, "dtb={ptr:#x}")?;
                } else {
                    writeln!(f, "dtb=absent")?;
                }
                if let Some(ptr) = self.0.table_presence.smbios {
                    writeln!(f, "smbios={ptr:#x}")?;
                } else {
                    writeln!(f, "smbios=absent")?;
                }
                if let Some(fb) = self.0.framebuffer {
                    writeln!(
                        f,
                        "framebuffer base={:#x} {}x{} stride={} format={}",
                        fb.base, fb.width, fb.height, fb.stride, fb.pixel_format
                    )?;
                } else {
                    writeln!(f, "framebuffer=unavailable")?;
                }
                Ok(())
            }
        }

        Summary(self)
    }
}

impl fmt::Display for BootObservation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.summary_lines())
    }
}

fn emit_seed_paths(report: &mut String, kind: &str, seeds: &[SeedPathObservation]) {
    let _ = writeln!(report, "{kind}_count={}", seeds.len());
    for (index, seed) in seeds.iter().enumerate() {
        let _ = writeln!(report, "{kind}[{index}].device_path={}", seed.device_path);
        let _ = writeln!(report, "{kind}[{index}].flags={:#x}", seed.flags);
    }
}

fn emit_accel_seed_drafts(report: &mut String, seeds: &[AccelSeedObservation]) {
    let _ = writeln!(report, "accel_seed_draft_count={}", seeds.len());
    for (index, accel) in seeds.iter().enumerate() {
        let _ = writeln!(report, "accel_seed_draft[{index}].source={}", accel.source);
        let _ = writeln!(
            report,
            "accel_seed_draft[{index}].transport={}",
            accel_transport_name(accel.seed.transport)
        );
        let _ = writeln!(
            report,
            "accel_seed_draft[{index}].topology_hint={}",
            accel_topology_name(accel.seed.topology_hint)
        );
        let _ = writeln!(
            report,
            "accel_seed_draft[{index}].memory_topology={}",
            memory_topology_name(accel.seed.memory_topology)
        );
        let _ = writeln!(
            report,
            "accel_seed_draft[{index}].platform_ready={}",
            accel.seed.platform_ready()
        );
        let _ = writeln!(
            report,
            "accel_seed_draft[{index}].vendor_id={}",
            accel.seed.vendor_id
        );
        let _ = writeln!(
            report,
            "accel_seed_draft[{index}].device_id={}",
            accel.seed.device_id
        );
        let _ = writeln!(
            report,
            "accel_seed_draft[{index}].class_code={:#x}",
            accel.seed.class_code
        );
        if let Some(path) = accel.raw_device_path.as_deref() {
            let _ = writeln!(report, "accel_seed_draft[{index}].raw_device_path={path}");
        } else {
            let _ = writeln!(report, "accel_seed_draft[{index}].raw_device_path=absent");
        }
    }
}

fn accel_transport_name(transport: AccelTransport) -> &'static str {
    match transport {
        AccelTransport::Unknown => "unknown",
        AccelTransport::Integrated => "integrated",
        AccelTransport::Platform => "platform",
        AccelTransport::Pci => "pci",
        AccelTransport::Fabric => "fabric",
    }
}

fn accel_topology_name(topology: AccelTopologyHint) -> &'static str {
    match topology {
        AccelTopologyHint::Unknown => "unknown",
        AccelTopologyHint::Uma => "uma",
        AccelTopologyHint::Discrete => "discrete",
        AccelTopologyHint::Partitionable => "partitionable",
        AccelTopologyHint::Linked => "linked",
    }
}

fn memory_topology_name(topology: MemoryTopologyClass) -> &'static str {
    match topology {
        MemoryTopologyClass::Unknown => "unknown",
        MemoryTopologyClass::Uma => "uma",
        MemoryTopologyClass::Discrete => "discrete",
        MemoryTopologyClass::Nvlink => "nvlink",
        MemoryTopologyClass::Mig => "mig",
    }
}

fn merge_seed_path(list: &mut Vec<SeedPathObservation>, device_path: String, flags: u32) {
    if let Some(existing) = list
        .iter_mut()
        .find(|entry| entry.device_path == device_path)
    {
        existing.flags |= flags;
        return;
    }

    list.push(SeedPathObservation { device_path, flags });
}

fn build_accel_seed_drafts(observation: &BootObservation) -> Vec<AccelSeedObservation> {
    let mut seeds = Vec::new();

    let mut seed = AccelSeedV1::empty();
    seed.transport = AccelTransport::Integrated;
    seed.topology_hint = AccelTopologyHint::Uma;
    seed.memory_topology = MemoryTopologyClass::Uma;

    let source = if observation.framebuffer.is_some() || !observation.display_paths.is_empty() {
        "spark_gop_anchor"
    } else {
        "spark_lane_draft"
    };

    seeds.push(AccelSeedObservation {
        seed,
        source,
        raw_device_path: observation
            .display_paths
            .first()
            .map(|entry| entry.device_path.clone()),
    });

    seeds
}

#[cfg(target_os = "uefi")]
fn read_u8_variable(name: &uefi::CStr16) -> Option<u8> {
    let mut buf = [0u8; 8];
    runtime::get_variable(name, &VariableVendor::GLOBAL_VARIABLE, &mut buf)
        .ok()
        .and_then(|(value, _)| value.first().copied())
}

#[cfg(target_os = "uefi")]
fn handle_device_path_text(handle: uefi::Handle) -> Option<String> {
    let device_path = uefi_boot::open_protocol_exclusive::<DevicePath>(handle).ok()?;
    let text = device_path
        .to_string(DisplayOnly(false), AllowShortcuts(true))
        .ok()?;
    Some(text.to_string())
}

#[cfg(target_os = "uefi")]
const fn pixel_format_name(format: PixelFormat) -> &'static str {
    match format {
        PixelFormat::Rgb => "rgb",
        PixelFormat::Bgr => "bgr",
        PixelFormat::Bitmask => "bitmask",
        PixelFormat::BltOnly => "blt-only",
    }
}
