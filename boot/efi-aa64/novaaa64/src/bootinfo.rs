#![allow(dead_code)]

#[cfg(target_os = "uefi")]
extern crate alloc;

use core::fmt;
#[cfg(any(target_os = "uefi", test))]
use core::fmt::Write as _;

#[cfg(target_os = "uefi")]
use alloc::format;
#[cfg(target_os = "uefi")]
use alloc::string::{String, ToString};
#[cfg(target_os = "uefi")]
use alloc::vec::Vec;
#[cfg(target_os = "uefi")]
use core::ptr::{self, NonNull};
#[cfg(target_os = "uefi")]
use core::slice;
use nova_fabric::{
    AccelSeedV1, AccelTopologyHint, AccelTransport, CpuArchitecture, MemoryTopologyClass,
    PlatformClass,
};
use nova_rt::InitCapsuleImage;
#[cfg(target_os = "uefi")]
use nova_rt::{
    BootSource, FramebufferFormat, NovaImageDigestV1, NovaPayloadEntryAbi, NovaPayloadKind,
    NovaVerificationInfoV1, PayloadImage,
};
use nova_rt::{
    NovaBootInfoV1, NovaBootInfoV2, NovaBootstrapPayloadDescriptorV1,
    NovaBootstrapUserWindowDescriptorV1, NovaDisplayPathDescriptorV1, NovaNetworkSeedV1,
    NovaStorageSeedV1,
};
use novaos_stage1::Stage1Status;
#[cfg(target_os = "uefi")]
use novaos_stage1::{Stage1Input, Stage1Plan, build_plan};
#[cfg(all(not(target_os = "uefi"), test))]
use std::string::String;
#[cfg(target_os = "uefi")]
use uefi::boot::{self, AllocateType, MemoryType};
#[cfg(target_os = "uefi")]
use uefi::fs::FileSystem;
#[cfg(target_os = "uefi")]
use uefi::mem::memory_map::{MemoryMap, MemoryMapOwned};
#[cfg(target_os = "uefi")]
use uefi::proto::console::gop::{GraphicsOutput, PixelFormat};
#[cfg(target_os = "uefi")]
use uefi::proto::device_path::DevicePath;
#[cfg(target_os = "uefi")]
use uefi::proto::device_path::text::{AllowShortcuts, DisplayOnly};
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
#[cfg(target_os = "uefi")]
pub const LOADER_REPORT_PATH: &str = r"\nova\loader\novaaa64-loader-report.txt";
#[cfg(target_os = "uefi")]
pub const LOADER_REPORT_FALLBACK_PATH: &str = r"\EFI\BOOT\novaaa64-loader-report.txt";
const EFI_PAGE_SIZE: usize = 4096;
const MEMORY_MAP_RESERVE_EXTRA_DESCRIPTORS: usize = 8;
const BOOTSTRAP_USER_WINDOW_BASE: u64 = 0x4000_0000;
const BOOTSTRAP_USER_WINDOW_MIN_SIZE: u64 = 0x20_000;
const BOOTSTRAP_USER_WINDOW_CONTEXT_RESERVE: u64 = 0x4000;
const BOOTSTRAP_USER_WINDOW_STACK_SIZE: u64 = 0x8000;
#[cfg(target_os = "uefi")]
type Stage1Entry = extern "C" fn(*const Stage1Plan) -> !;
#[cfg(target_os = "uefi")]
const DISPLAY_FLAG_GOP_HANDLE: u32 = 1 << 0;
#[cfg(target_os = "uefi")]
const STORAGE_FLAG_FILESYSTEM_HANDLE: u32 = 1 << 0;
#[cfg(target_os = "uefi")]
const STORAGE_FLAG_BLOCK_HANDLE: u32 = 1 << 1;
#[cfg(target_os = "uefi")]
const NETWORK_FLAG_SIMPLE_NETWORK_HANDLE: u32 = 1 << 0;

#[cfg(target_os = "uefi")]
#[derive(Clone, Debug, Eq, PartialEq)]
struct SeedPathText {
    device_path: String,
    flags: u32,
}

#[cfg(target_os = "uefi")]
#[derive(Default)]
struct BootInfoV2SeedPaths {
    display_path: Option<SeedPathText>,
    storage_seeds: Vec<SeedPathText>,
    network_seeds: Vec<SeedPathText>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct BootInfoV2SeedLayout {
    display_path_ptr: u64,
    display_path_len: u32,
    display_path_flags: u32,
    storage_seeds_ptr: u64,
    storage_seed_count: u32,
    network_seeds_ptr: u64,
    network_seed_count: u32,
    accel_seeds_ptr: u64,
    accel_seed_count: u32,
    bootstrap_payload: NovaBootstrapPayloadDescriptorV1,
    bootstrap_user_window: NovaBootstrapUserWindowDescriptorV1,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PayloadRegion {
    pub path: &'static str,
    pub base: u64,
    pub len: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct LoaderPlan {
    pub boot_info: NovaBootInfoV1,
    pub boot_info_v2_draft: NovaBootInfoV2,
    pub boot_info_v2: Option<PayloadRegion>,
    pub display_path_v2: Option<PayloadRegion>,
    pub storage_seeds_v2: Option<PayloadRegion>,
    pub network_seeds_v2: Option<PayloadRegion>,
    pub accel_seed_v2: Option<PayloadRegion>,
    pub kernel_image_digest: Option<PayloadRegion>,
    pub verification_info: Option<PayloadRegion>,
    pub memory_map: Option<PayloadRegion>,
    pub stage1_plan: Option<PayloadRegion>,
    pub stage1_image: Option<PayloadRegion>,
    pub kernel_image: Option<PayloadRegion>,
    pub init_capsule: Option<PayloadRegion>,
    pub loader_log: Option<PayloadRegion>,
}

impl LoaderPlan {
    pub fn unknown() -> Self {
        Self {
            boot_info: NovaBootInfoV1::new(),
            boot_info_v2_draft: NovaBootInfoV2::new(),
            boot_info_v2: None,
            display_path_v2: None,
            storage_seeds_v2: None,
            network_seeds_v2: None,
            accel_seed_v2: None,
            kernel_image_digest: None,
            verification_info: None,
            memory_map: None,
            stage1_plan: None,
            stage1_image: None,
            kernel_image: None,
            init_capsule: None,
            loader_log: None,
        }
    }

    pub fn summary(&self) -> impl fmt::Display + '_ {
        struct Summary<'a>(&'a LoaderPlan);

        impl fmt::Display for Summary<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                writeln!(f, "NovaOS stage0 loader")?;
                writeln!(f, "boot_info_magic={:#x}", self.0.boot_info.magic)?;
                writeln!(f, "boot_info_version={}", self.0.boot_info.version)?;
                writeln!(f, "flags={:#x}", self.0.boot_info.flags)?;
                writeln!(
                    f,
                    "boot_info_v2_magic={:#x}",
                    self.0.boot_info_v2_draft.magic
                )?;
                writeln!(
                    f,
                    "boot_info_v2_version={}",
                    self.0.boot_info_v2_draft.version
                )?;
                writeln!(
                    f,
                    "boot_info_v2_valid={}",
                    self.0.boot_info_v2_draft.is_valid()
                )?;
                writeln!(
                    f,
                    "boot_info_v2_cpu_arch={}",
                    self.0.boot_info_v2_draft.cpu_arch as u16
                )?;
                writeln!(
                    f,
                    "boot_info_v2_platform_class={}",
                    self.0.boot_info_v2_draft.platform_class as u16
                )?;
                writeln!(
                    f,
                    "boot_info_v2_memory_topology={}",
                    self.0.boot_info_v2_draft.memory_topology_class as u16
                )?;
                writeln!(
                    f,
                    "boot_info_v2_display_path_len={}",
                    self.0.boot_info_v2_draft.display_path.device_path_len
                )?;
                writeln!(
                    f,
                    "boot_info_v2_storage_seed_count={}",
                    self.0.boot_info_v2_draft.storage_seed_count
                )?;
                writeln!(
                    f,
                    "boot_info_v2_network_seed_count={}",
                    self.0.boot_info_v2_draft.network_seed_count
                )?;
                writeln!(
                    f,
                    "boot_info_v2_accel_seed_count={}",
                    self.0.boot_info_v2_draft.accel_seed_count
                )?;
                writeln!(
                    f,
                    "firmware_revision={}",
                    self.0.boot_info.firmware_revision
                )?;
                writeln!(
                    f,
                    "secure_boot_state={}",
                    self.0.boot_info.secure_boot_state
                )?;
                writeln!(f, "boot_source={}", self.0.boot_info.boot_source as u8)?;
                writeln!(f, "current_el={}", self.0.boot_info.current_el)?;
                writeln!(
                    f,
                    "config_table_count={}",
                    self.0.boot_info.config_table_count
                )?;
                writeln!(
                    f,
                    "memory_map_entries={}",
                    self.0.boot_info.memory_map_entries
                )?;
                writeln!(
                    f,
                    "memory_map_desc_size={}",
                    self.0.boot_info.memory_map_desc_size
                )?;
                writeln!(f, "acpi_rsdp_ptr={:#x}", self.0.boot_info.acpi_rsdp_ptr)?;
                writeln!(f, "dtb_ptr={:#x}", self.0.boot_info.dtb_ptr)?;
                writeln!(f, "smbios_ptr={:#x}", self.0.boot_info.smbios_ptr)?;
                writeln!(
                    f,
                    "framebuffer_base={:#x}",
                    self.0.boot_info.framebuffer_base
                )?;
                writeln!(
                    f,
                    "framebuffer={}x{} stride={} format={}",
                    self.0.boot_info.framebuffer_width,
                    self.0.boot_info.framebuffer_height,
                    self.0.boot_info.framebuffer_stride,
                    self.0.boot_info.framebuffer_format as u32
                )?;
                describe_region(f, "boot_info_v2", self.0.boot_info_v2)?;
                describe_region(f, "display_path_v2", self.0.display_path_v2)?;
                describe_region(f, "storage_seeds_v2", self.0.storage_seeds_v2)?;
                describe_region(f, "network_seeds_v2", self.0.network_seeds_v2)?;
                describe_region(f, "accel_seed_v2", self.0.accel_seed_v2)?;
                describe_region(f, "kernel_image_digest", self.0.kernel_image_digest)?;
                describe_region(f, "verification_info", self.0.verification_info)?;
                describe_region(f, "memory_map", self.0.memory_map)?;
                describe_region(f, "stage1_plan", self.0.stage1_plan)?;
                describe_region(f, "stage1_image", self.0.stage1_image)?;
                describe_region(f, "kernel_image", self.0.kernel_image)?;
                describe_region(f, "init_capsule", self.0.init_capsule)?;
                describe_region(f, "loader_log", self.0.loader_log)?;
                Ok(())
            }
        }

        Summary(self)
    }

    #[cfg(any(target_os = "uefi", test))]
    pub fn structured_handoff_report(&self, stage1_plan_ready: bool) -> String {
        let mut report = String::new();
        let framebuffer_present =
            self.boot_info.framebuffer_width != 0 && self.boot_info.framebuffer_height != 0;

        let _ = writeln!(report, "report_kind=novaaa64_loader_handoff_report");
        let _ = writeln!(report, "report_version=1");
        let _ = writeln!(report, "stage1_plan_ready={stage1_plan_ready}");
        let _ = writeln!(report, "boot_info_valid={}", self.boot_info.is_valid());
        let _ = writeln!(report, "boot_info_magic={:#x}", self.boot_info.magic);
        let _ = writeln!(report, "boot_info_version={}", self.boot_info.version);
        let _ = writeln!(report, "flags={:#x}", self.boot_info.flags);
        let _ = writeln!(
            report,
            "boot_info_v2_valid={}",
            self.boot_info_v2_draft.is_valid()
        );
        let _ = writeln!(
            report,
            "boot_info_v2_cpu_arch={}",
            self.boot_info_v2_draft.cpu_arch as u16
        );
        let _ = writeln!(
            report,
            "boot_info_v2_platform_class={}",
            self.boot_info_v2_draft.platform_class as u16
        );
        let _ = writeln!(
            report,
            "boot_info_v2_memory_topology={}",
            self.boot_info_v2_draft.memory_topology_class as u16
        );
        let _ = writeln!(
            report,
            "boot_info_v2_display_path_len={}",
            self.boot_info_v2_draft.display_path.device_path_len
        );
        let _ = writeln!(
            report,
            "boot_info_v2_storage_seed_count={}",
            self.boot_info_v2_draft.storage_seed_count
        );
        let _ = writeln!(
            report,
            "boot_info_v2_network_seed_count={}",
            self.boot_info_v2_draft.network_seed_count
        );
        let _ = writeln!(
            report,
            "boot_info_v2_accel_seed_count={}",
            self.boot_info_v2_draft.accel_seed_count
        );
        let _ = writeln!(
            report,
            "boot_info_v2_bootstrap_payload_present={}",
            !self.boot_info_v2_draft.bootstrap_payload.is_empty()
        );
        let _ = writeln!(
            report,
            "boot_info_v2_bootstrap_payload_size={}",
            self.boot_info_v2_draft.bootstrap_payload.image_len
        );
        let _ = writeln!(
            report,
            "boot_info_v2_bootstrap_user_window_present={}",
            !self.boot_info_v2_draft.bootstrap_user_window.is_empty()
        );
        let _ = writeln!(
            report,
            "boot_info_v2_bootstrap_user_window_base={:#x}",
            self.boot_info_v2_draft.bootstrap_user_window.base
        );
        let _ = writeln!(
            report,
            "boot_info_v2_bootstrap_user_window_size={}",
            self.boot_info_v2_draft.bootstrap_user_window.len
        );
        let _ = writeln!(
            report,
            "boot_info_v2_bootstrap_user_stack_size={}",
            self.boot_info_v2_draft.bootstrap_user_window.stack_size
        );
        let _ = writeln!(
            report,
            "firmware_revision={}",
            self.boot_info.firmware_revision
        );
        let _ = writeln!(
            report,
            "secure_boot_state={}",
            self.boot_info.secure_boot_state
        );
        let _ = writeln!(report, "boot_source={}", self.boot_info.boot_source as u8);
        let _ = writeln!(report, "current_el={}", self.boot_info.current_el);
        let _ = writeln!(
            report,
            "config_table_count={}",
            self.boot_info.config_table_count
        );
        let _ = writeln!(
            report,
            "memory_map_entries={}",
            self.boot_info.memory_map_entries
        );
        let _ = writeln!(
            report,
            "memory_map_desc_size={}",
            self.boot_info.memory_map_desc_size
        );
        let _ = writeln!(report, "acpi_rsdp_ptr={:#x}", self.boot_info.acpi_rsdp_ptr);
        let _ = writeln!(report, "dtb_ptr={:#x}", self.boot_info.dtb_ptr);
        let _ = writeln!(report, "smbios_ptr={:#x}", self.boot_info.smbios_ptr);
        let _ = writeln!(report, "framebuffer_present={framebuffer_present}");
        let _ = writeln!(
            report,
            "framebuffer_base={:#x}",
            self.boot_info.framebuffer_base
        );
        let _ = writeln!(
            report,
            "framebuffer_width={}",
            self.boot_info.framebuffer_width
        );
        let _ = writeln!(
            report,
            "framebuffer_height={}",
            self.boot_info.framebuffer_height
        );
        let _ = writeln!(
            report,
            "framebuffer_stride={}",
            self.boot_info.framebuffer_stride
        );
        let _ = writeln!(
            report,
            "framebuffer_format={}",
            self.boot_info.framebuffer_format as u32
        );
        emit_report_region(&mut report, "boot_info_v2", self.boot_info_v2);
        emit_report_region(&mut report, "display_path_v2", self.display_path_v2);
        emit_report_region(&mut report, "storage_seeds_v2", self.storage_seeds_v2);
        emit_report_region(&mut report, "network_seeds_v2", self.network_seeds_v2);
        emit_report_region(&mut report, "accel_seed_v2", self.accel_seed_v2);
        emit_report_region(&mut report, "kernel_image_digest", self.kernel_image_digest);
        emit_report_region(&mut report, "verification_info", self.verification_info);
        emit_report_region(&mut report, "memory_map", self.memory_map);
        emit_report_region(&mut report, "stage1_plan", self.stage1_plan);
        emit_report_region(&mut report, "stage1_image", self.stage1_image);
        emit_report_region(&mut report, "kernel_image", self.kernel_image);
        emit_report_region(&mut report, "init_capsule", self.init_capsule);
        emit_report_region(&mut report, "loader_log", self.loader_log);

        report
    }
}

pub struct LoaderHandoff {
    pub plan: LoaderPlan,
    #[cfg(target_os = "uefi")]
    buffers: LoaderBuffers,
}

impl LoaderHandoff {
    pub fn prepare(plan: LoaderPlan) -> Self {
        Self {
            plan,
            #[cfg(target_os = "uefi")]
            buffers: LoaderBuffers::empty(),
        }
    }

    #[cfg(target_os = "uefi")]
    pub fn from_uefi() -> Self {
        let mut buffers = LoaderBuffers::empty();
        let mut boot_info = NovaBootInfoV1::new();
        boot_info.firmware_vendor_ptr = system::firmware_vendor().as_ptr() as u64;
        boot_info.firmware_revision = system::firmware_revision();
        boot_info.secure_boot_state = read_secure_boot_state();
        boot_info.boot_source = BootSource::Unknown;
        boot_info.current_el = read_current_el();

        let (
            config_tables_ptr,
            config_table_count,
            acpi_rsdp_ptr,
            dtb_ptr,
            smbios_ptr,
            table_flags,
        ) = system::with_config_table(|tables| {
            let mut flags = 0u32;
            let mut acpi_rsdp_ptr = 0u64;
            let mut dtb_ptr = 0u64;
            let mut smbios_ptr = 0u64;

            for table in tables {
                let address = table.address as u64;

                if table.guid == cfg::ACPI_GUID || table.guid == cfg::ACPI2_GUID {
                    acpi_rsdp_ptr = address;
                    flags |= NovaBootInfoV1::FLAG_HAS_ACPI_RSDP;
                }

                if table.guid == cfg::SMBIOS_GUID || table.guid == cfg::SMBIOS3_GUID {
                    smbios_ptr = address;
                    flags |= NovaBootInfoV1::FLAG_HAS_SMBIOS;
                }

                if table.guid == DEVICE_TREE_GUID {
                    dtb_ptr = address;
                    flags |= NovaBootInfoV1::FLAG_HAS_DTB;
                }
            }

            (
                tables.as_ptr() as u64,
                tables.len() as u32,
                acpi_rsdp_ptr,
                dtb_ptr,
                smbios_ptr,
                flags,
            )
        });

        boot_info.config_tables_ptr = config_tables_ptr;
        boot_info.config_table_count = config_table_count;
        boot_info.acpi_rsdp_ptr = acpi_rsdp_ptr;
        boot_info.dtb_ptr = dtb_ptr;
        boot_info.smbios_ptr = smbios_ptr;
        boot_info.flags |= table_flags;

        if let Ok(memory_map) = boot::memory_map(MemoryType::LOADER_DATA) {
            boot_info.memory_map_entries = memory_map.entries().count() as u32;
            boot_info.memory_map_desc_size = memory_map.meta().desc_size as u32;
        }

        if let Ok(handle) = uefi_boot::get_handle_for_protocol::<GraphicsOutput>() {
            if let Ok(mut gop) = uefi_boot::open_protocol_exclusive::<GraphicsOutput>(handle) {
                let mode = gop.current_mode_info();
                let (width, height) = mode.resolution();
                let stride = mode.stride();

                boot_info.framebuffer_base = gop.frame_buffer().as_mut_ptr() as u64;
                boot_info.framebuffer_width = width as u32;
                boot_info.framebuffer_height = height as u32;
                boot_info.framebuffer_stride = stride as u32;
                boot_info.framebuffer_format = pixel_format_code(mode.pixel_format());
                boot_info.set_flag(NovaBootInfoV1::FLAG_HAS_FRAMEBUFFER);
            }
        }

        let mut plan = LoaderPlan {
            boot_info,
            boot_info_v2_draft: NovaBootInfoV2::new(),
            boot_info_v2: None,
            display_path_v2: None,
            storage_seeds_v2: None,
            network_seeds_v2: None,
            accel_seed_v2: None,
            kernel_image_digest: None,
            verification_info: None,
            memory_map: None,
            stage1_plan: None,
            stage1_image: None,
            kernel_image: None,
            init_capsule: None,
            loader_log: None,
        };

        if let Ok(fs_proto) = boot::get_image_file_system(boot::image_handle()) {
            let mut fs = FileSystem::new(fs_proto);

            plan.stage1_image = load_payload(
                &mut fs,
                cstr16!(r"\nova\stage1.bin"),
                "stage1.bin",
                MemoryType::LOADER_CODE,
                &mut buffers.stage1_image,
            );
            plan.kernel_image = load_payload(
                &mut fs,
                cstr16!(r"\nova\kernel.bin"),
                "kernel.bin",
                MemoryType::LOADER_CODE,
                &mut buffers.kernel_image,
            );
            plan.init_capsule = load_payload(
                &mut fs,
                cstr16!(r"\nova\init.capsule"),
                "init.capsule",
                MemoryType::LOADER_DATA,
                &mut buffers.init_capsule,
            );
        }

        if let Some(region) = plan.init_capsule {
            plan.boot_info.init_capsule_ptr = region.base;
            plan.boot_info.init_capsule_len = region.len as u64;
        }

        if let Some(kernel_digest) = buffers
            .kernel_image
            .as_ref()
            .and_then(|image| build_kernel_image_digest(image.as_slice()))
            .and_then(PersistentObject::new)
        {
            plan.kernel_image_digest = Some(kernel_digest.region("kernel.digest"));
            plan.boot_info.kernel_image_hash_ptr = kernel_digest.as_ptr() as u64;
            plan.boot_info
                .set_flag(NovaBootInfoV1::FLAG_HAS_KERNEL_IMAGE_DIGEST);
            buffers.kernel_image_digest = Some(kernel_digest);
        }

        if let Some(verification_info) =
            build_verification_info(&buffers).and_then(PersistentObject::new)
        {
            plan.verification_info = Some(verification_info.region("verification.info"));
            plan.boot_info.verification_info_ptr = verification_info.as_ptr() as u64;
            plan.boot_info
                .set_flag(NovaBootInfoV1::FLAG_HAS_VERIFICATION_INFO);
            buffers.verification_info = Some(verification_info);
        }

        if let Some(memory_map) = reserve_memory_map_storage() {
            plan.memory_map = Some(memory_map.region("memory_map"));
            plan.boot_info.memory_map_ptr = memory_map.as_ptr() as u64;
            buffers.memory_map = Some(memory_map);
        }

        if let Some(stage1_plan) = PersistentObject::new(Stage1Plan::empty()) {
            plan.stage1_plan = Some(stage1_plan.region("stage1.plan"));
            buffers.stage1_plan = Some(stage1_plan);
        }

        let mut boot_info_v2_seeds = persist_boot_info_v2_seed_paths(
            collect_boot_info_v2_seed_paths(),
            &mut plan,
            &mut buffers,
        );
        boot_info_v2_seeds.bootstrap_payload = build_bootstrap_payload_descriptor(
            buffers.init_capsule.as_ref().map(PersistentBytes::as_slice),
        );
        boot_info_v2_seeds.bootstrap_user_window =
            build_bootstrap_user_window_descriptor(boot_info_v2_seeds.bootstrap_payload);

        if let Some(accel_seed_v2) = PersistentObject::new(build_accel_seed_v2()) {
            plan.accel_seed_v2 = Some(accel_seed_v2.region("bootinfo_v2.accel_seed"));
            buffers.accel_seed_v2 = Some(accel_seed_v2);
        }

        boot_info_v2_seeds.accel_seeds_ptr = buffers
            .accel_seed_v2
            .as_ref()
            .map(|seed| seed.as_ptr() as u64)
            .unwrap_or(0);
        boot_info_v2_seeds.accel_seed_count = u32::from(buffers.accel_seed_v2.is_some());
        plan.boot_info_v2_draft = build_boot_info_v2_draft(&plan, &boot_info_v2_seeds);

        let log_text = build_loader_log(&plan);
        if let Some(loader_log) = PersistentBytes::copy_from(log_text.as_bytes()) {
            plan.loader_log = Some(loader_log.region("loader.log"));
            plan.boot_info.loader_log_ptr = loader_log.as_ptr() as u64;
            plan.boot_info.set_flag(NovaBootInfoV1::FLAG_HAS_LOADER_LOG);
            buffers.loader_log = Some(loader_log);
        }

        plan.boot_info_v2_draft = build_boot_info_v2_draft(&plan, &boot_info_v2_seeds);
        if let Some(boot_info_v2) = PersistentObject::new(plan.boot_info_v2_draft) {
            plan.boot_info_v2 = Some(boot_info_v2.region("bootinfo_v2"));
            buffers.boot_info_v2 = Some(boot_info_v2);
        }

        let boot_info_slot =
            PersistentObject::new(plan.boot_info).expect("stage0 boot info allocation failed");
        buffers.boot_info = Some(boot_info_slot);

        Self { plan, buffers }
    }

    #[cfg(target_os = "uefi")]
    pub fn structured_handoff_report(&self) -> String {
        self.plan
            .structured_handoff_report(self.build_stage1_plan().is_ok())
    }

    #[cfg(target_os = "uefi")]
    pub fn persist_handoff_report(&self) -> Option<String> {
        let report = self.structured_handoff_report();
        let fs_proto = boot::get_image_file_system(boot::image_handle()).ok()?;
        let mut fs = FileSystem::new(fs_proto);

        if fs.create_dir_all(cstr16!(r"\nova\loader")).is_ok()
            && fs
                .write(
                    cstr16!(r"\nova\loader\novaaa64-loader-report.txt"),
                    report.as_bytes(),
                )
                .is_ok()
        {
            return Some(String::from(LOADER_REPORT_PATH));
        }

        if fs
            .write(
                cstr16!(r"\EFI\BOOT\novaaa64-loader-report.txt"),
                report.as_bytes(),
            )
            .is_ok()
        {
            return Some(String::from(LOADER_REPORT_FALLBACK_PATH));
        }

        None
    }

    pub fn summary(&self) -> impl fmt::Display + '_ {
        self.plan.summary()
    }

    pub fn validate(&self) -> Result<(), LoaderError> {
        if !self.plan.boot_info.is_valid() {
            return Err(LoaderError::InvalidBootInfo);
        }

        #[cfg(target_os = "uefi")]
        if self.buffers.stage1_image.is_none() {
            return Err(LoaderError::MissingStage1Image);
        }

        #[cfg(target_os = "uefi")]
        if self.buffers.kernel_image.is_none() {
            return Err(LoaderError::MissingKernelImage);
        }

        #[cfg(target_os = "uefi")]
        if self.buffers.kernel_image_digest.is_none() {
            return Err(LoaderError::MissingKernelImageDigest);
        }

        #[cfg(target_os = "uefi")]
        if self.buffers.verification_info.is_none() {
            return Err(LoaderError::MissingVerificationInfo);
        }

        #[cfg(target_os = "uefi")]
        if self
            .buffers
            .stage1_image
            .as_ref()
            .and_then(|image| {
                PayloadImage::parse_kind_abi(
                    image.as_slice(),
                    NovaPayloadKind::Stage1,
                    NovaPayloadEntryAbi::Stage1Plan,
                )
            })
            .is_none()
        {
            return Err(LoaderError::InvalidStage1Image);
        }

        #[cfg(target_os = "uefi")]
        if self
            .buffers
            .kernel_image
            .as_ref()
            .and_then(|image| {
                PayloadImage::parse_kind_abi(
                    image.as_slice(),
                    NovaPayloadKind::Kernel,
                    NovaPayloadEntryAbi::BootInfoV2Sidecar,
                )
            })
            .is_none()
        {
            return Err(LoaderError::InvalidKernelImage);
        }

        #[cfg(target_os = "uefi")]
        if !self
            .buffers
            .kernel_image
            .as_ref()
            .zip(self.buffers.kernel_image_digest.as_ref())
            .map(|(image, digest)| kernel_image_digest_matches(image.as_slice(), digest.as_ref()))
            .unwrap_or(false)
        {
            return Err(LoaderError::InvalidKernelImageDigest);
        }

        #[cfg(target_os = "uefi")]
        if !self
            .buffers
            .verification_info
            .as_ref()
            .map(|verification| verification_matches_expected(&self.buffers, verification.as_ref()))
            .unwrap_or(false)
        {
            return Err(LoaderError::InvalidVerificationInfo);
        }

        #[cfg(target_os = "uefi")]
        if self.buffers.stage1_plan.is_none() {
            return Err(LoaderError::MissingStage1PlanStorage);
        }

        #[cfg(target_os = "uefi")]
        if self.buffers.memory_map.is_none() {
            return Err(LoaderError::MissingMemoryMapStorage);
        }

        #[cfg(target_os = "uefi")]
        self.build_stage1_plan()?;

        #[cfg(target_os = "uefi")]
        if let Some(boot_info_v2) = self.buffers.boot_info_v2.as_ref() {
            let display_path = self
                .buffers
                .display_path_text_v2
                .as_ref()
                .map(PersistentBytes::as_slice);
            let storage_seeds = self
                .buffers
                .storage_seeds_v2
                .as_ref()
                .map(PersistentSlice::as_slice);
            let network_seeds = self
                .buffers
                .network_seeds_v2
                .as_ref()
                .map(PersistentSlice::as_slice);
            let accel_seed = self
                .buffers
                .accel_seed_v2
                .as_ref()
                .map(PersistentObject::as_ref);
            let init_capsule = self
                .buffers
                .init_capsule
                .as_ref()
                .map(PersistentBytes::as_slice);
            if !validate_boot_info_v2(
                boot_info_v2.as_ref(),
                display_path,
                storage_seeds,
                network_seeds,
                accel_seed,
                init_capsule,
            ) {
                return Err(LoaderError::InvalidBootInfoV2);
            }
        }

        Ok(())
    }

    #[cfg(target_os = "uefi")]
    pub fn exit_boot_services_and_prepare_stage1(mut self) -> PostExitStage {
        self.sync_boot_info_storage();

        let memory_map = unsafe { boot::exit_boot_services(Some(MemoryType::LOADER_DATA)) };
        let memory_map_ptr = self.persist_final_memory_map(&memory_map);

        {
            let boot_info = self.boot_info_mut();
            boot_info.memory_map_entries = memory_map.entries().count() as u32;
            boot_info.memory_map_desc_size = memory_map.meta().desc_size as u32;
            boot_info.memory_map_ptr = memory_map_ptr;
            self.plan.boot_info = *boot_info;
        }

        if let Some(boot_info_v2) = self.boot_info_v2_mut() {
            boot_info_v2.memory_map_entries = memory_map.entries().count() as u32;
            boot_info_v2.memory_map_desc_size = memory_map.meta().desc_size as u32;
            boot_info_v2.memory_map_ptr = memory_map_ptr;
            self.plan.boot_info_v2_draft = *boot_info_v2;
        }

        let stage1_plan = self
            .build_stage1_plan()
            .expect("stage0 post-exit stage1 plan must be valid");
        *self.stage1_plan_mut() = stage1_plan;

        PostExitStage {
            memory_map,
            buffers: self.buffers,
        }
    }

    #[cfg(target_os = "uefi")]
    fn build_stage1_plan(&self) -> Result<Stage1Plan, LoaderError> {
        let boot_info = self
            .buffers
            .boot_info
            .as_ref()
            .map(PersistentObject::as_ref)
            .ok_or(LoaderError::MissingBootInfoStorage)?;
        let boot_info_v2 = self
            .buffers
            .boot_info_v2
            .as_ref()
            .map(PersistentObject::as_ref);
        let kernel_image = self
            .buffers
            .kernel_image
            .as_ref()
            .map(PersistentBytes::as_slice)
            .ok_or(LoaderError::MissingKernelImage)?;
        let init_capsule = self
            .buffers
            .init_capsule
            .as_ref()
            .map(PersistentBytes::as_slice);

        let input = Stage1Input {
            boot_info,
            boot_info_v2,
            kernel_image,
            init_capsule,
            secure_boot: boot_info.secure_boot_enabled(),
        };

        build_plan(&input).map_err(LoaderError::Stage1)
    }

    #[cfg(target_os = "uefi")]
    fn boot_info_mut(&mut self) -> &mut NovaBootInfoV1 {
        self.buffers
            .boot_info
            .as_mut()
            .expect("boot info storage must exist")
            .as_mut()
    }

    #[cfg(target_os = "uefi")]
    fn sync_boot_info_storage(&mut self) {
        *self.boot_info_mut() = self.plan.boot_info;
    }

    #[cfg(target_os = "uefi")]
    fn boot_info_v2_mut(&mut self) -> Option<&mut NovaBootInfoV2> {
        self.buffers
            .boot_info_v2
            .as_mut()
            .map(PersistentObject::as_mut)
    }

    #[cfg(target_os = "uefi")]
    fn stage1_plan_mut(&mut self) -> &mut Stage1Plan {
        self.buffers
            .stage1_plan
            .as_mut()
            .expect("stage1 plan storage must exist")
            .as_mut()
    }

    #[cfg(target_os = "uefi")]
    fn persist_final_memory_map(&mut self, memory_map: &MemoryMapOwned) -> u64 {
        let final_map_len = memory_map.meta().map_size;
        let storage = self
            .buffers
            .memory_map
            .as_mut()
            .expect("memory map storage must exist");
        let persistent = storage.as_mut_slice();

        assert!(
            final_map_len <= persistent.len(),
            "persistent memory map storage is too small"
        );

        persistent[..final_map_len].copy_from_slice(&memory_map.buffer()[..final_map_len]);
        if final_map_len < persistent.len() {
            persistent[final_map_len..].fill(0);
        }

        storage.as_ptr() as u64
    }
}

#[cfg(target_os = "uefi")]
pub struct PostExitStage {
    memory_map: MemoryMapOwned,
    buffers: LoaderBuffers,
}

#[cfg(target_os = "uefi")]
impl PostExitStage {
    pub fn run(self) -> ! {
        let PostExitStage {
            memory_map,
            buffers,
        } = self;

        let _ = &memory_map;
        let stage1_plan = buffers
            .stage1_plan
            .as_ref()
            .expect("stage1 plan storage must exist");
        let stage1_payload = buffers
            .stage1_image
            .as_ref()
            .expect("stage1 payload must exist");
        let kernel_payload = buffers
            .kernel_image
            .as_ref()
            .expect("kernel payload must exist");

        mask_interrupts();
        trace_post_exit_stage0();
        buffers.clean_handoff_data_cache();
        sync_instruction_cache(stage1_payload.as_ptr(), stage1_payload.len());
        sync_instruction_cache(kernel_payload.as_ptr(), kernel_payload.len());

        let stage1_entry = Self::stage1_entry(stage1_payload);
        stage1_entry(stage1_plan.as_ptr())
    }

    fn stage1_entry(stage1_payload: &PersistentBytes) -> Stage1Entry {
        let image = PayloadImage::parse_kind_abi(
            stage1_payload.as_slice(),
            NovaPayloadKind::Stage1,
            NovaPayloadEntryAbi::Stage1Plan,
        )
        .expect("stage1 payload header must be valid");
        let entry = image.entry_addr(stage1_payload.as_ptr() as u64) as usize;
        unsafe { core::mem::transmute::<usize, Stage1Entry>(entry) }
    }
}

#[cfg(target_os = "uefi")]
struct LoaderBuffers {
    boot_info: Option<PersistentObject<NovaBootInfoV1>>,
    boot_info_v2: Option<PersistentObject<NovaBootInfoV2>>,
    display_path_text_v2: Option<PersistentBytes>,
    storage_seed_texts_v2: Vec<PersistentBytes>,
    storage_seeds_v2: Option<PersistentSlice<NovaStorageSeedV1>>,
    network_seed_texts_v2: Vec<PersistentBytes>,
    network_seeds_v2: Option<PersistentSlice<NovaNetworkSeedV1>>,
    accel_seed_v2: Option<PersistentObject<AccelSeedV1>>,
    kernel_image_digest: Option<PersistentObject<NovaImageDigestV1>>,
    verification_info: Option<PersistentObject<NovaVerificationInfoV1>>,
    memory_map: Option<PersistentBytes>,
    stage1_plan: Option<PersistentObject<Stage1Plan>>,
    stage1_image: Option<PersistentBytes>,
    kernel_image: Option<PersistentBytes>,
    init_capsule: Option<PersistentBytes>,
    loader_log: Option<PersistentBytes>,
}

#[cfg(target_os = "uefi")]
impl LoaderBuffers {
    fn empty() -> Self {
        Self {
            boot_info: None,
            boot_info_v2: None,
            display_path_text_v2: None,
            storage_seed_texts_v2: Vec::new(),
            storage_seeds_v2: None,
            network_seed_texts_v2: Vec::new(),
            network_seeds_v2: None,
            accel_seed_v2: None,
            kernel_image_digest: None,
            verification_info: None,
            memory_map: None,
            stage1_plan: None,
            stage1_image: None,
            kernel_image: None,
            init_capsule: None,
            loader_log: None,
        }
    }

    fn clean_handoff_data_cache(&self) {
        if let Some(boot_info) = self.boot_info.as_ref() {
            clean_data_cache(
                boot_info.as_ptr() as *const u8,
                core::mem::size_of::<NovaBootInfoV1>(),
            );
        }
        if let Some(boot_info_v2) = self.boot_info_v2.as_ref() {
            clean_data_cache(
                boot_info_v2.as_ptr() as *const u8,
                core::mem::size_of::<NovaBootInfoV2>(),
            );
        }
        if let Some(display_path) = self.display_path_text_v2.as_ref() {
            clean_data_cache(display_path.as_ptr(), display_path.len());
        }
        for text in &self.storage_seed_texts_v2 {
            clean_data_cache(text.as_ptr(), text.len());
        }
        if let Some(storage_seeds) = self.storage_seeds_v2.as_ref() {
            clean_data_cache(
                storage_seeds.as_ptr() as *const u8,
                storage_seeds.len() * core::mem::size_of::<NovaStorageSeedV1>(),
            );
        }
        for text in &self.network_seed_texts_v2 {
            clean_data_cache(text.as_ptr(), text.len());
        }
        if let Some(network_seeds) = self.network_seeds_v2.as_ref() {
            clean_data_cache(
                network_seeds.as_ptr() as *const u8,
                network_seeds.len() * core::mem::size_of::<NovaNetworkSeedV1>(),
            );
        }
        if let Some(accel_seed) = self.accel_seed_v2.as_ref() {
            clean_data_cache(
                accel_seed.as_ptr() as *const u8,
                core::mem::size_of::<AccelSeedV1>(),
            );
        }
        if let Some(kernel_image_digest) = self.kernel_image_digest.as_ref() {
            clean_data_cache(
                kernel_image_digest.as_ptr() as *const u8,
                core::mem::size_of::<NovaImageDigestV1>(),
            );
        }
        if let Some(verification_info) = self.verification_info.as_ref() {
            clean_data_cache(
                verification_info.as_ptr() as *const u8,
                core::mem::size_of::<NovaVerificationInfoV1>(),
            );
        }
        if let Some(memory_map) = self.memory_map.as_ref() {
            clean_data_cache(memory_map.as_ptr(), memory_map.len());
        }
        if let Some(stage1_plan) = self.stage1_plan.as_ref() {
            clean_data_cache(
                stage1_plan.as_ptr() as *const u8,
                core::mem::size_of::<Stage1Plan>(),
            );
        }
        if let Some(init_capsule) = self.init_capsule.as_ref() {
            clean_data_cache(init_capsule.as_ptr(), init_capsule.len());
        }
        if let Some(loader_log) = self.loader_log.as_ref() {
            clean_data_cache(loader_log.as_ptr(), loader_log.len());
        }
    }
}

#[cfg(target_os = "uefi")]
struct PersistentSlice<T> {
    ptr: NonNull<T>,
    len: usize,
    pages: usize,
}

#[cfg(target_os = "uefi")]
impl<T: Copy> PersistentSlice<T> {
    fn copy_from(values: &[T]) -> Option<Self> {
        if values.is_empty() {
            return None;
        }

        let byte_len = core::mem::size_of_val(values);
        let pages = page_count(byte_len);
        let ptr = allocate_loader_pages(pages, MemoryType::LOADER_DATA)?.cast::<T>();
        unsafe {
            ptr::copy_nonoverlapping(values.as_ptr(), ptr.as_ptr(), values.len());
        }
        Some(Self {
            ptr,
            len: values.len(),
            pages,
        })
    }

    fn as_ptr(&self) -> *const T {
        self.ptr.as_ptr()
    }

    fn len(&self) -> usize {
        self.len
    }

    fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }

    fn region(&self, path: &'static str) -> PayloadRegion {
        PayloadRegion {
            path,
            base: self.ptr.as_ptr() as u64,
            len: self.len * core::mem::size_of::<T>(),
        }
    }
}

#[cfg(target_os = "uefi")]
struct PersistentObject<T> {
    ptr: NonNull<T>,
    pages: usize,
}

#[cfg(target_os = "uefi")]
impl<T> PersistentObject<T> {
    fn new(value: T) -> Option<Self> {
        let pages = page_count(core::mem::size_of::<T>());
        let ptr = allocate_loader_pages(pages, MemoryType::LOADER_DATA)?.cast::<T>();
        unsafe {
            ptr.as_ptr().write(value);
        }
        Some(Self { ptr, pages })
    }

    fn as_ref(&self) -> &T {
        unsafe { self.ptr.as_ref() }
    }

    fn as_mut(&mut self) -> &mut T {
        unsafe { self.ptr.as_mut() }
    }

    fn as_ptr(&self) -> *const T {
        self.ptr.as_ptr()
    }

    fn region(&self, path: &'static str) -> PayloadRegion {
        PayloadRegion {
            path,
            base: self.ptr.as_ptr() as u64,
            len: core::mem::size_of::<T>(),
        }
    }
}

#[cfg(target_os = "uefi")]
struct PersistentBytes {
    ptr: NonNull<u8>,
    len: usize,
    pages: usize,
}

#[cfg(target_os = "uefi")]
impl PersistentBytes {
    fn copy_from(bytes: &[u8]) -> Option<Self> {
        Self::copy_from_with_type(bytes, MemoryType::LOADER_DATA)
    }

    fn copy_code(bytes: &[u8]) -> Option<Self> {
        Self::copy_from_with_type(bytes, MemoryType::LOADER_CODE)
    }

    fn zeroed(byte_len: usize, memory_type: MemoryType) -> Option<Self> {
        let pages = page_count(byte_len);
        let ptr = allocate_loader_pages(pages, memory_type)?;
        unsafe {
            ptr.as_ptr().write_bytes(0, pages * EFI_PAGE_SIZE);
        }
        Some(Self {
            ptr,
            len: byte_len,
            pages,
        })
    }

    fn copy_from_with_type(bytes: &[u8], memory_type: MemoryType) -> Option<Self> {
        let pages = page_count(bytes.len());
        let ptr = allocate_loader_pages(pages, memory_type)?;
        unsafe {
            ptr.as_ptr().write_bytes(0, pages * EFI_PAGE_SIZE);
            if !bytes.is_empty() {
                ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.as_ptr(), bytes.len());
            }
        }
        Some(Self {
            ptr,
            len: bytes.len(),
            pages,
        })
    }

    fn as_ptr(&self) -> *const u8 {
        self.ptr.as_ptr()
    }

    fn len(&self) -> usize {
        self.len
    }

    fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }

    fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }

    fn region(&self, path: &'static str) -> PayloadRegion {
        PayloadRegion {
            path,
            base: self.ptr.as_ptr() as u64,
            len: self.len,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoaderError {
    InvalidBootInfo,
    InvalidBootInfoV2,
    MissingBootInfoStorage,
    MissingKernelImageDigest,
    InvalidKernelImageDigest,
    MissingVerificationInfo,
    InvalidVerificationInfo,
    MissingMemoryMapStorage,
    MissingStage1PlanStorage,
    InvalidStage1Image,
    MissingStage1Image,
    InvalidKernelImage,
    MissingKernelImage,
    Stage1(Stage1Status),
}

fn describe_region(
    f: &mut fmt::Formatter<'_>,
    label: &str,
    region: Option<PayloadRegion>,
) -> fmt::Result {
    if let Some(region) = region {
        writeln!(
            f,
            "{label}=true path={} base={:#x} len={}",
            region.path, region.base, region.len
        )
    } else {
        writeln!(f, "{label}=false")
    }
}

#[cfg(any(target_os = "uefi", test))]
fn emit_report_region(report: &mut String, label: &str, region: Option<PayloadRegion>) {
    let _ = writeln!(report, "{label}.present={}", region.is_some());
    if let Some(region) = region {
        let _ = writeln!(report, "{label}.path={}", region.path);
        let _ = writeln!(report, "{label}.base={:#x}", region.base);
        let _ = writeln!(report, "{label}.len={}", region.len);
    }
}

#[cfg(target_os = "uefi")]
fn load_payload(
    fs: &mut FileSystem,
    path: &uefi::CStr16,
    label: &'static str,
    memory_type: MemoryType,
    slot: &mut Option<PersistentBytes>,
) -> Option<PayloadRegion> {
    let bytes = fs.read(path).ok()?;
    let persistent = match memory_type {
        MemoryType::LOADER_CODE => PersistentBytes::copy_code(&bytes)?,
        _ => PersistentBytes::copy_from_with_type(&bytes, memory_type)?,
    };
    let region = persistent.region(label);
    *slot = Some(persistent);
    Some(region)
}

#[cfg(target_os = "uefi")]
fn build_loader_log(plan: &LoaderPlan) -> String {
    format!(
        "NovaOS stage0 log\nflags={:#x}\nconfig_tables={}\nmemory_map_entries={}\nbootinfo_v2_valid={}\nbootinfo_v2_platform_class={}\nbootinfo_v2_memory_topology={}\nbootinfo_v2_display_path={}\nbootinfo_v2_storage_seeds={}\nbootinfo_v2_network_seeds={}\nbootinfo_v2_accel_seeds={}\nbootinfo_v2_bootstrap_payload={}\nbootinfo_v2_bootstrap_payload_size={}\nbootinfo_v2_bootstrap_user_window={}\nbootinfo_v2_bootstrap_user_window_base={:#x}\nbootinfo_v2_bootstrap_user_window_size={}\nbootinfo_v2_bootstrap_user_stack_size={}\nkernel_image_digest={}\nverification_info={}\nmemory_map_storage={}\nstage1_plan_storage={}\nstage1={}\nkernel={}\ninit_capsule={}\n",
        plan.boot_info.flags,
        plan.boot_info.config_table_count,
        plan.boot_info.memory_map_entries,
        plan.boot_info_v2_draft.is_valid(),
        plan.boot_info_v2_draft.platform_class as u16,
        plan.boot_info_v2_draft.memory_topology_class as u16,
        plan.boot_info_v2_draft.display_path.device_path_len != 0,
        plan.boot_info_v2_draft.storage_seed_count,
        plan.boot_info_v2_draft.network_seed_count,
        plan.boot_info_v2_draft.accel_seed_count,
        !plan.boot_info_v2_draft.bootstrap_payload.is_empty(),
        plan.boot_info_v2_draft.bootstrap_payload.image_len,
        !plan.boot_info_v2_draft.bootstrap_user_window.is_empty(),
        plan.boot_info_v2_draft.bootstrap_user_window.base,
        plan.boot_info_v2_draft.bootstrap_user_window.len,
        plan.boot_info_v2_draft.bootstrap_user_window.stack_size,
        plan.kernel_image_digest.is_some(),
        plan.verification_info.is_some(),
        plan.memory_map.is_some(),
        plan.stage1_plan.is_some(),
        plan.stage1_image.is_some(),
        plan.kernel_image.is_some(),
        plan.init_capsule.is_some(),
    )
}

#[cfg(target_os = "uefi")]
fn build_kernel_image_digest(image: &[u8]) -> Option<NovaImageDigestV1> {
    if image.is_empty() {
        return None;
    }

    Some(NovaImageDigestV1::from_bytes_sha256(image))
}

#[cfg(target_os = "uefi")]
fn kernel_image_digest_matches(image: &[u8], digest: &NovaImageDigestV1) -> bool {
    build_kernel_image_digest(image)
        .map(|expected| expected == *digest && digest.is_valid())
        .unwrap_or(false)
}

#[cfg(target_os = "uefi")]
fn build_verification_info(buffers: &LoaderBuffers) -> Option<NovaVerificationInfoV1> {
    let mut info = NovaVerificationInfoV1::new();

    if let Some(stage1_image) = buffers.stage1_image.as_ref() {
        info.stage1_image_size = stage1_image.len() as u64;
        info.set_flag(NovaVerificationInfoV1::FLAG_STAGE1_PAYLOAD_PRESENT);
        if PayloadImage::parse_kind_abi(
            stage1_image.as_slice(),
            NovaPayloadKind::Stage1,
            NovaPayloadEntryAbi::Stage1Plan,
        )
        .is_some()
        {
            info.set_flag(NovaVerificationInfoV1::FLAG_STAGE1_PAYLOAD_VERIFIED);
        }
    }

    if let Some(kernel_image) = buffers.kernel_image.as_ref() {
        info.kernel_image_size = kernel_image.len() as u64;
        info.set_flag(NovaVerificationInfoV1::FLAG_KERNEL_PAYLOAD_PRESENT);
        if PayloadImage::parse_kind_abi(
            kernel_image.as_slice(),
            NovaPayloadKind::Kernel,
            NovaPayloadEntryAbi::BootInfoV2Sidecar,
        )
        .is_some()
        {
            info.set_flag(NovaVerificationInfoV1::FLAG_KERNEL_PAYLOAD_VERIFIED);
        }
    }

    if buffers.init_capsule.is_some() {
        info.set_flag(NovaVerificationInfoV1::FLAG_INIT_CAPSULE_PRESENT);
    }

    if let Some(digest) = buffers.kernel_image_digest.as_ref() {
        info.set_flag(NovaVerificationInfoV1::FLAG_KERNEL_DIGEST_PRESENT);
        if buffers
            .kernel_image
            .as_ref()
            .map(|image| kernel_image_digest_matches(image.as_slice(), digest.as_ref()))
            .unwrap_or(false)
        {
            info.set_flag(NovaVerificationInfoV1::FLAG_KERNEL_DIGEST_VERIFIED);
        }
    }

    info.is_valid().then_some(info)
}

#[cfg(target_os = "uefi")]
fn verification_matches_expected(
    buffers: &LoaderBuffers,
    verification: &NovaVerificationInfoV1,
) -> bool {
    build_verification_info(buffers)
        .map(|expected| expected == *verification)
        .unwrap_or(false)
}

#[cfg(target_os = "uefi")]
fn read_secure_boot_state() -> u8 {
    let mut buf = [0u8; 8];
    match runtime::get_variable(
        cstr16!("SecureBoot"),
        &VariableVendor::GLOBAL_VARIABLE,
        &mut buf,
    ) {
        Ok((value, _)) => {
            if value.first().copied().unwrap_or(0) == 0 {
                NovaBootInfoV1::SECURE_BOOT_DISABLED
            } else {
                NovaBootInfoV1::SECURE_BOOT_ENABLED
            }
        }
        Err(_) => NovaBootInfoV1::SECURE_BOOT_UNKNOWN,
    }
}

#[cfg(all(target_os = "uefi", target_arch = "aarch64"))]
fn read_current_el() -> u8 {
    let current_el: u64;
    unsafe {
        core::arch::asm!("mrs {}, CurrentEL", out(reg) current_el);
    }
    ((current_el >> 2) & 0b11) as u8
}

#[cfg(not(all(target_os = "uefi", target_arch = "aarch64")))]
fn read_current_el() -> u8 {
    0
}

#[cfg(target_os = "uefi")]
fn collect_boot_info_v2_seed_paths() -> BootInfoV2SeedPaths {
    let mut seed_paths = BootInfoV2SeedPaths::default();

    if let Ok(handle) = uefi_boot::get_handle_for_protocol::<GraphicsOutput>() {
        if let Some(device_path) = handle_device_path_text(handle) {
            seed_paths.display_path = Some(SeedPathText {
                device_path,
                flags: DISPLAY_FLAG_GOP_HANDLE,
            });
        }
    }

    if let Ok(handles) = boot::find_handles::<SimpleFileSystem>() {
        for handle in handles {
            if let Some(device_path) = handle_device_path_text(handle) {
                merge_seed_path(
                    &mut seed_paths.storage_seeds,
                    device_path,
                    STORAGE_FLAG_FILESYSTEM_HANDLE,
                );
            }
        }
    }

    if let Ok(handles) = boot::find_handles::<BlockIO>() {
        for handle in handles {
            if let Some(device_path) = handle_device_path_text(handle) {
                merge_seed_path(
                    &mut seed_paths.storage_seeds,
                    device_path,
                    STORAGE_FLAG_BLOCK_HANDLE,
                );
            }
        }
    }

    if let Ok(handles) = boot::find_handles::<SimpleNetwork>() {
        for handle in handles {
            if let Some(device_path) = handle_device_path_text(handle) {
                merge_seed_path(
                    &mut seed_paths.network_seeds,
                    device_path,
                    NETWORK_FLAG_SIMPLE_NETWORK_HANDLE,
                );
            }
        }
    }

    seed_paths
}

#[cfg(target_os = "uefi")]
fn persist_boot_info_v2_seed_paths(
    seed_paths: BootInfoV2SeedPaths,
    plan: &mut LoaderPlan,
    buffers: &mut LoaderBuffers,
) -> BootInfoV2SeedLayout {
    let mut layout = BootInfoV2SeedLayout::default();

    if let Some(display_path) = seed_paths.display_path {
        if let Some(bytes) = PersistentBytes::copy_from(display_path.device_path.as_bytes()) {
            layout.display_path_ptr = bytes.as_ptr() as u64;
            layout.display_path_len = bytes.len() as u32;
            layout.display_path_flags = display_path.flags;
            plan.display_path_v2 = Some(bytes.region("bootinfo_v2.display_path"));
            buffers.display_path_text_v2 = Some(bytes);
        }
    }

    let mut storage_descriptors = Vec::with_capacity(seed_paths.storage_seeds.len());
    for seed in seed_paths.storage_seeds {
        if let Some(bytes) = PersistentBytes::copy_from(seed.device_path.as_bytes()) {
            storage_descriptors.push(NovaStorageSeedV1 {
                device_path_ptr: bytes.as_ptr() as u64,
                device_path_len: bytes.len() as u32,
                flags: seed.flags,
            });
            buffers.storage_seed_texts_v2.push(bytes);
        }
    }
    if let Some(storage_seeds) = PersistentSlice::copy_from(storage_descriptors.as_slice()) {
        layout.storage_seeds_ptr = storage_seeds.as_ptr() as u64;
        layout.storage_seed_count = storage_seeds.len() as u32;
        plan.storage_seeds_v2 = Some(storage_seeds.region("bootinfo_v2.storage_seeds"));
        buffers.storage_seeds_v2 = Some(storage_seeds);
    }

    let mut network_descriptors = Vec::with_capacity(seed_paths.network_seeds.len());
    for seed in seed_paths.network_seeds {
        if let Some(bytes) = PersistentBytes::copy_from(seed.device_path.as_bytes()) {
            network_descriptors.push(NovaNetworkSeedV1 {
                device_path_ptr: bytes.as_ptr() as u64,
                device_path_len: bytes.len() as u32,
                flags: seed.flags,
            });
            buffers.network_seed_texts_v2.push(bytes);
        }
    }
    if let Some(network_seeds) = PersistentSlice::copy_from(network_descriptors.as_slice()) {
        layout.network_seeds_ptr = network_seeds.as_ptr() as u64;
        layout.network_seed_count = network_seeds.len() as u32;
        plan.network_seeds_v2 = Some(network_seeds.region("bootinfo_v2.network_seeds"));
        buffers.network_seeds_v2 = Some(network_seeds);
    }

    layout
}

#[cfg(target_os = "uefi")]
fn merge_seed_path(list: &mut Vec<SeedPathText>, device_path: String, flags: u32) {
    if let Some(existing) = list
        .iter_mut()
        .find(|entry| entry.device_path == device_path)
    {
        existing.flags |= flags;
        return;
    }

    list.push(SeedPathText { device_path, flags });
}

#[cfg(target_os = "uefi")]
fn handle_device_path_text(handle: uefi::Handle) -> Option<String> {
    let device_path = uefi_boot::open_protocol_exclusive::<DevicePath>(handle).ok()?;
    let text = device_path
        .to_string(DisplayOnly(false), AllowShortcuts(true))
        .ok()?;
    Some(text.to_string())
}

fn build_accel_seed_v2() -> AccelSeedV1 {
    let mut seed = AccelSeedV1::empty();
    seed.transport = AccelTransport::Integrated;
    seed.topology_hint = AccelTopologyHint::Uma;
    seed.memory_topology = MemoryTopologyClass::Uma;
    seed
}

#[cfg(any(target_os = "uefi", test))]
fn build_bootstrap_payload_descriptor(
    init_capsule: Option<&[u8]>,
) -> NovaBootstrapPayloadDescriptorV1 {
    let Some(capsule) = init_capsule.and_then(InitCapsuleImage::parse) else {
        return NovaBootstrapPayloadDescriptorV1::empty();
    };
    let Some(payload) = capsule.bootstrap_service_payload() else {
        return NovaBootstrapPayloadDescriptorV1::empty();
    };

    let image = payload.image_bytes();
    let image_base = image.as_ptr() as u64;
    NovaBootstrapPayloadDescriptorV1 {
        image_ptr: image_base,
        image_len: image.len() as u64,
        load_base: payload.load_base(image_base),
        load_size: payload.load_size(),
        entry_point: payload.entry_addr(image_base),
    }
}

#[cfg(all(not(target_os = "uefi"), not(test)))]
fn build_bootstrap_payload_descriptor(
    _init_capsule: Option<&[u8]>,
) -> NovaBootstrapPayloadDescriptorV1 {
    NovaBootstrapPayloadDescriptorV1::empty()
}

fn build_bootstrap_user_window_descriptor(
    payload: NovaBootstrapPayloadDescriptorV1,
) -> NovaBootstrapUserWindowDescriptorV1 {
    if payload.is_empty() || !payload.is_valid() {
        return NovaBootstrapUserWindowDescriptorV1::empty();
    }

    let Some(image_size) = align_up_bootstrap_user_window(payload.load_size) else {
        return NovaBootstrapUserWindowDescriptorV1::empty();
    };
    let Some(required_len) = image_size
        .checked_add(BOOTSTRAP_USER_WINDOW_CONTEXT_RESERVE)
        .and_then(|len| len.checked_add(BOOTSTRAP_USER_WINDOW_STACK_SIZE))
    else {
        return NovaBootstrapUserWindowDescriptorV1::empty();
    };
    let len = if required_len > BOOTSTRAP_USER_WINDOW_MIN_SIZE {
        required_len
    } else {
        BOOTSTRAP_USER_WINDOW_MIN_SIZE
    };

    NovaBootstrapUserWindowDescriptorV1 {
        base: BOOTSTRAP_USER_WINDOW_BASE,
        len,
        stack_size: BOOTSTRAP_USER_WINDOW_STACK_SIZE,
        page_size: NovaBootstrapUserWindowDescriptorV1::PAGE_SIZE_4K,
        flags: 0,
    }
}

fn align_up_bootstrap_user_window(value: u64) -> Option<u64> {
    let page_mask = NovaBootstrapUserWindowDescriptorV1::PAGE_SIZE_4K as u64 - 1;
    value.checked_add(page_mask).map(|value| value & !page_mask)
}

fn build_boot_info_v2_draft(plan: &LoaderPlan, seeds: &BootInfoV2SeedLayout) -> NovaBootInfoV2 {
    let mut info = NovaBootInfoV2::new();
    info.cpu_arch = CpuArchitecture::Arm64;
    info.platform_class = PlatformClass::SparkUma;
    info.memory_topology_class = MemoryTopologyClass::Uma;
    info.secure_boot_state = plan.boot_info.secure_boot_state;
    info.boot_source = plan.boot_info.boot_source;
    info.current_el = plan.boot_info.current_el;
    info.firmware_vendor_ptr = plan.boot_info.firmware_vendor_ptr;
    info.firmware_revision = plan.boot_info.firmware_revision;
    info.memory_map_ptr = plan.boot_info.memory_map_ptr;
    info.memory_map_entries = plan.boot_info.memory_map_entries;
    info.memory_map_desc_size = plan.boot_info.memory_map_desc_size;
    info.config_tables_ptr = plan.boot_info.config_tables_ptr;
    info.config_table_count = plan.boot_info.config_table_count;
    info.acpi_rsdp_ptr = plan.boot_info.acpi_rsdp_ptr;
    info.dtb_ptr = plan.boot_info.dtb_ptr;
    info.smbios_ptr = plan.boot_info.smbios_ptr;
    info.framebuffer.base = plan.boot_info.framebuffer_base;
    info.framebuffer.width = plan.boot_info.framebuffer_width;
    info.framebuffer.height = plan.boot_info.framebuffer_height;
    info.framebuffer.stride = plan.boot_info.framebuffer_stride;
    info.framebuffer.format = plan.boot_info.framebuffer_format;
    info.display_path = NovaDisplayPathDescriptorV1 {
        device_path_ptr: seeds.display_path_ptr,
        device_path_len: seeds.display_path_len,
        flags: seeds.display_path_flags,
    };
    info.storage_seeds_ptr = seeds.storage_seeds_ptr;
    info.storage_seed_count = seeds.storage_seed_count;
    info.network_seeds_ptr = seeds.network_seeds_ptr;
    info.network_seed_count = seeds.network_seed_count;
    info.accel_seeds_ptr = seeds.accel_seeds_ptr;
    info.accel_seed_count = seeds.accel_seed_count;
    info.init_capsule_ptr = plan.boot_info.init_capsule_ptr;
    info.init_capsule_len = plan.boot_info.init_capsule_len;
    info.loader_log_ptr = plan.boot_info.loader_log_ptr;
    info.loader_log_len = plan.loader_log.map_or(0, |region| region.len as u64);
    info.kernel_image_hash_ptr = plan.boot_info.kernel_image_hash_ptr;
    info.bootstrap_payload = seeds.bootstrap_payload;
    info.bootstrap_user_window = if seeds.bootstrap_user_window.is_empty() {
        build_bootstrap_user_window_descriptor(seeds.bootstrap_payload)
    } else {
        seeds.bootstrap_user_window
    };
    info
}

fn validate_boot_info_v2(
    info: &NovaBootInfoV2,
    display_path: Option<&[u8]>,
    storage_seeds: Option<&[NovaStorageSeedV1]>,
    network_seeds: Option<&[NovaNetworkSeedV1]>,
    accel_seed: Option<&AccelSeedV1>,
    init_capsule: Option<&[u8]>,
) -> bool {
    if !info.is_valid() {
        return false;
    }

    if !validate_path_descriptor(&info.display_path, display_path) {
        return false;
    }

    if !validate_seed_slice(
        info.storage_seeds_ptr,
        info.storage_seed_count,
        storage_seeds,
        |seed| seed.device_path_ptr != 0 && seed.device_path_len != 0,
    ) {
        return false;
    }

    if !validate_seed_slice(
        info.network_seeds_ptr,
        info.network_seed_count,
        network_seeds,
        |seed| seed.device_path_ptr != 0 && seed.device_path_len != 0,
    ) {
        return false;
    }

    let accel_valid = if info.accel_seed_count == 0 {
        info.accel_seeds_ptr == 0
    } else {
        if info.accel_seed_count != 1 {
            return false;
        }

        let Some(accel_seed) = accel_seed else {
            return false;
        };

        info.accel_seeds_ptr == accel_seed as *const AccelSeedV1 as u64
            && accel_seed.platform_ready()
    };

    accel_valid && validate_bootstrap_payload_descriptor(&info.bootstrap_payload, init_capsule)
}

fn validate_bootstrap_payload_descriptor(
    descriptor: &NovaBootstrapPayloadDescriptorV1,
    init_capsule: Option<&[u8]>,
) -> bool {
    if !descriptor.is_valid() {
        return false;
    }

    let Some(init_capsule) = init_capsule else {
        return descriptor.is_empty();
    };
    let Some(capsule) = InitCapsuleImage::parse(init_capsule) else {
        return false;
    };
    let Some(payload) = capsule.bootstrap_service_payload() else {
        return descriptor.is_empty();
    };

    let image = payload.image_bytes();
    let image_base = image.as_ptr() as u64;
    descriptor.image_ptr == image_base
        && descriptor.image_len == image.len() as u64
        && descriptor.load_base == payload.load_base(image_base)
        && descriptor.load_size == payload.load_size()
        && descriptor.entry_point == payload.entry_addr(image_base)
}

fn validate_path_descriptor(
    descriptor: &NovaDisplayPathDescriptorV1,
    bytes: Option<&[u8]>,
) -> bool {
    if descriptor.device_path_len == 0 {
        return descriptor.device_path_ptr == 0 && descriptor.flags == 0;
    }

    let Some(bytes) = bytes else {
        return false;
    };

    descriptor.device_path_ptr == bytes.as_ptr() as u64
        && descriptor.device_path_len as usize == bytes.len()
        && descriptor.flags != 0
}

fn validate_seed_slice<T>(
    ptr: u64,
    count: u32,
    seeds: Option<&[T]>,
    validate_seed: impl Fn(&T) -> bool,
) -> bool {
    if count == 0 {
        return ptr == 0;
    }

    let Some(seeds) = seeds else {
        return false;
    };

    ptr == seeds.as_ptr() as u64 && count as usize == seeds.len() && seeds.iter().all(validate_seed)
}

#[cfg(target_os = "uefi")]
const fn pixel_format_code(format: PixelFormat) -> FramebufferFormat {
    match format {
        PixelFormat::Rgb => FramebufferFormat::Rgbx8888,
        PixelFormat::Bgr => FramebufferFormat::Bgrx8888,
        PixelFormat::Bitmask | PixelFormat::BltOnly => FramebufferFormat::Unknown,
    }
}

#[cfg(target_os = "uefi")]
fn page_count(byte_len: usize) -> usize {
    let bytes = byte_len.max(1);
    (bytes + (EFI_PAGE_SIZE - 1)) / EFI_PAGE_SIZE
}

#[cfg(target_os = "uefi")]
fn allocate_loader_pages(pages: usize, memory_type: MemoryType) -> Option<NonNull<u8>> {
    boot::allocate_pages(AllocateType::AnyPages, memory_type, pages).ok()
}

#[cfg(target_os = "uefi")]
fn reserve_memory_map_storage() -> Option<PersistentBytes> {
    let memory_map = boot::memory_map(MemoryType::LOADER_DATA).ok()?;
    let reserve_len =
        memory_map_reserve_len(memory_map.meta().map_size, memory_map.meta().desc_size);
    PersistentBytes::zeroed(reserve_len, MemoryType::LOADER_DATA)
}

fn memory_map_reserve_len(map_size: usize, desc_size: usize) -> usize {
    map_size + (desc_size * MEMORY_MAP_RESERVE_EXTRA_DESCRIPTORS)
}

#[cfg(test)]
mod loader_tests {
    use super::{
        BootInfoV2SeedLayout, LoaderPlan, PayloadRegion, build_accel_seed_v2,
        build_boot_info_v2_draft, build_bootstrap_payload_descriptor,
        build_bootstrap_user_window_descriptor, memory_map_reserve_len, validate_boot_info_v2,
    };
    use nova_fabric::{AccelTransport, CpuArchitecture, MemoryTopologyClass, PlatformClass};
    use nova_rt::{
        FramebufferFormat, InitCapsuleImage, NovaBootInfoV1, NovaInitCapsuleCapabilityV1,
        NovaInitCapsuleHeaderV1, NovaNetworkSeedV1, NovaPayloadEntryAbi, NovaPayloadHeaderV1,
        NovaPayloadKind, NovaStorageSeedV1, encode_init_capsule_service_name, sha256_digest_bytes,
    };

    #[test]
    fn memory_map_reserve_len_adds_descriptor_headroom() {
        assert_eq!(memory_map_reserve_len(4096, 48), 4096 + (48 * 8));
    }

    #[test]
    fn accel_seed_v2_matches_spark_uma_lane() {
        let seed = build_accel_seed_v2();
        assert_eq!(seed.transport, AccelTransport::Integrated);
        assert!(seed.platform_ready());
    }

    #[test]
    fn boot_info_v2_draft_tracks_current_loader_basics() {
        let mut plan = LoaderPlan::unknown();
        plan.boot_info = NovaBootInfoV1::new();
        plan.boot_info.firmware_revision = 7;
        plan.boot_info.current_el = 2;
        plan.boot_info.memory_map_ptr = 0x1000;
        plan.boot_info.memory_map_entries = 4;
        plan.boot_info.memory_map_desc_size = 48;
        plan.boot_info.config_tables_ptr = 0x2000;
        plan.boot_info.config_table_count = 3;
        plan.boot_info.acpi_rsdp_ptr = 0x3000;
        plan.boot_info.framebuffer_base = 0x4000;
        plan.boot_info.framebuffer_width = 1920;
        plan.boot_info.framebuffer_height = 1080;
        plan.boot_info.framebuffer_stride = 1920;
        plan.boot_info.framebuffer_format = FramebufferFormat::Rgbx8888;
        plan.boot_info.init_capsule_ptr = 0x5000;
        plan.boot_info.init_capsule_len = 64;
        plan.boot_info.kernel_image_hash_ptr = 0x6000;
        plan.boot_info.loader_log_ptr = 0x7000;
        plan.loader_log = Some(PayloadRegion {
            path: "loader.log",
            base: 0x7000,
            len: 32,
        });

        let seeds = BootInfoV2SeedLayout {
            display_path_ptr: 0x7100,
            display_path_len: 16,
            display_path_flags: 1,
            storage_seeds_ptr: 0x7200,
            storage_seed_count: 2,
            network_seeds_ptr: 0x7300,
            network_seed_count: 1,
            accel_seeds_ptr: 0x8000,
            accel_seed_count: 1,
            ..BootInfoV2SeedLayout::default()
        };
        let info = build_boot_info_v2_draft(&plan, &seeds);
        assert!(info.is_valid());
        assert_eq!(info.cpu_arch, CpuArchitecture::Arm64);
        assert_eq!(info.platform_class, PlatformClass::SparkUma);
        assert_eq!(info.memory_topology_class, MemoryTopologyClass::Uma);
        assert_eq!(info.memory_map_ptr, 0x1000);
        assert_eq!(info.loader_log_len, 32);
        assert_eq!(info.display_path.device_path_ptr, 0x7100);
        assert_eq!(info.display_path.device_path_len, 16);
        assert_eq!(info.display_path.flags, 1);
        assert_eq!(info.storage_seeds_ptr, 0x7200);
        assert_eq!(info.storage_seed_count, 2);
        assert_eq!(info.network_seeds_ptr, 0x7300);
        assert_eq!(info.network_seed_count, 1);
        assert_eq!(info.accel_seeds_ptr, 0x8000);
        assert_eq!(info.accel_seed_count, 1);
        assert!(info.bootstrap_user_window.is_empty());
        assert!(info.framebuffer_present());
    }

    #[test]
    fn bootstrap_payload_descriptor_tracks_embedded_bootstrap_image() {
        let init_capsule = build_init_capsule_with_payload();
        let descriptor = build_bootstrap_payload_descriptor(Some(init_capsule.as_slice()));
        let capsule = InitCapsuleImage::parse(init_capsule.as_slice()).expect("capsule");
        let payload = capsule
            .bootstrap_service_payload()
            .expect("bootstrap payload");
        let image = payload.image_bytes();
        let image_base = image.as_ptr() as u64;

        assert_eq!(descriptor.image_ptr, image_base);
        assert_eq!(descriptor.image_len, image.len() as u64);
        assert_eq!(descriptor.load_base, payload.load_base(image_base));
        assert_eq!(descriptor.load_size, payload.load_size());
        assert_eq!(descriptor.entry_point, payload.entry_addr(image_base));
    }

    #[test]
    fn bootstrap_user_window_descriptor_tracks_payload_window_policy() {
        let init_capsule = build_init_capsule_with_payload();
        let payload = build_bootstrap_payload_descriptor(Some(init_capsule.as_slice()));
        let user_window = build_bootstrap_user_window_descriptor(payload);

        assert!(user_window.is_valid());
        assert!(!user_window.is_empty());
        assert_eq!(user_window.base, 0x4000_0000);
        assert_eq!(user_window.len, 0x20_000);
        assert_eq!(user_window.stack_size, 0x8000);
        assert_eq!(user_window.page_size, 4096);

        let info = build_boot_info_v2_draft(
            &LoaderPlan::unknown(),
            &BootInfoV2SeedLayout {
                bootstrap_payload: payload,
                bootstrap_user_window: user_window,
                ..BootInfoV2SeedLayout::default()
            },
        );
        assert!(info.is_valid());
        assert_eq!(info.bootstrap_user_window, user_window);

        let inferred_info = build_boot_info_v2_draft(
            &LoaderPlan::unknown(),
            &BootInfoV2SeedLayout {
                bootstrap_payload: payload,
                ..BootInfoV2SeedLayout::default()
            },
        );
        assert_eq!(inferred_info.bootstrap_user_window, user_window);
    }

    #[test]
    fn validate_boot_info_v2_requires_matching_platform_ready_seed() {
        let seed = build_accel_seed_v2();
        let display_path = b"PciRoot(0x0)/Pci(0x1,0x0)";
        let init_capsule = build_init_capsule_with_payload();
        let storage_seeds = [NovaStorageSeedV1 {
            device_path_ptr: 0x9000,
            device_path_len: 12,
            flags: 1,
        }];
        let network_seeds = [NovaNetworkSeedV1 {
            device_path_ptr: 0xA000,
            device_path_len: 12,
            flags: 1,
        }];
        let mut info = build_boot_info_v2_draft(
            &LoaderPlan::unknown(),
            &BootInfoV2SeedLayout {
                display_path_ptr: display_path.as_ptr() as u64,
                display_path_len: display_path.len() as u32,
                display_path_flags: 1,
                storage_seeds_ptr: storage_seeds.as_ptr() as u64,
                storage_seed_count: storage_seeds.len() as u32,
                network_seeds_ptr: network_seeds.as_ptr() as u64,
                network_seed_count: network_seeds.len() as u32,
                accel_seeds_ptr: &seed as *const _ as u64,
                accel_seed_count: 1,
                bootstrap_payload: build_bootstrap_payload_descriptor(Some(
                    init_capsule.as_slice(),
                )),
                ..BootInfoV2SeedLayout::default()
            },
        );
        assert!(validate_boot_info_v2(
            &info,
            Some(display_path),
            Some(&storage_seeds),
            Some(&network_seeds),
            Some(&seed),
            Some(init_capsule.as_slice()),
        ));

        info.accel_seeds_ptr = 0;
        assert!(!validate_boot_info_v2(
            &info,
            Some(display_path),
            Some(&storage_seeds),
            Some(&network_seeds),
            Some(&seed),
            Some(init_capsule.as_slice()),
        ));
    }

    #[test]
    fn validate_boot_info_v2_rejects_missing_display_path_bytes() {
        let info = build_boot_info_v2_draft(
            &LoaderPlan::unknown(),
            &BootInfoV2SeedLayout {
                display_path_ptr: 0x1234,
                display_path_len: 8,
                ..BootInfoV2SeedLayout::default()
            },
        );

        assert!(!validate_boot_info_v2(&info, None, None, None, None, None));
    }

    #[test]
    fn validate_boot_info_v2_requires_bootstrap_payload_bytes_when_descriptor_present() {
        let seed = build_accel_seed_v2();
        let init_capsule = build_init_capsule_with_payload();
        let info = build_boot_info_v2_draft(
            &LoaderPlan::unknown(),
            &BootInfoV2SeedLayout {
                accel_seeds_ptr: &seed as *const _ as u64,
                accel_seed_count: 1,
                bootstrap_payload: build_bootstrap_payload_descriptor(Some(
                    init_capsule.as_slice(),
                )),
                ..BootInfoV2SeedLayout::default()
            },
        );

        assert!(!validate_boot_info_v2(
            &info,
            None,
            None,
            None,
            Some(&seed),
            None,
        ));
    }

    #[test]
    fn validate_boot_info_v2_rejects_mismatched_bootstrap_payload_descriptor() {
        let seed = build_accel_seed_v2();
        let init_capsule = build_init_capsule_with_payload();
        let mut info = build_boot_info_v2_draft(
            &LoaderPlan::unknown(),
            &BootInfoV2SeedLayout {
                accel_seeds_ptr: &seed as *const _ as u64,
                accel_seed_count: 1,
                bootstrap_payload: build_bootstrap_payload_descriptor(Some(
                    init_capsule.as_slice(),
                )),
                ..BootInfoV2SeedLayout::default()
            },
        );
        info.bootstrap_payload.entry_point += 4;

        assert!(!validate_boot_info_v2(
            &info,
            None,
            None,
            None,
            Some(&seed),
            Some(init_capsule.as_slice()),
        ));
    }

    #[test]
    fn structured_handoff_report_tracks_v2_and_payload_readiness() {
        let init_capsule = build_init_capsule_with_payload();
        let mut plan = LoaderPlan::unknown();
        plan.boot_info = NovaBootInfoV1::new();
        plan.boot_info.firmware_revision = 7;
        plan.boot_info.current_el = 2;
        plan.boot_info.framebuffer_base = 0x4000;
        plan.boot_info.framebuffer_width = 1920;
        plan.boot_info.framebuffer_height = 1080;
        plan.boot_info.framebuffer_stride = 1920;
        plan.boot_info.framebuffer_format = FramebufferFormat::Rgbx8888;
        plan.boot_info_v2_draft = build_boot_info_v2_draft(
            &plan,
            &BootInfoV2SeedLayout {
                display_path_ptr: 0x7100,
                display_path_len: 16,
                display_path_flags: 1,
                storage_seeds_ptr: 0x7200,
                storage_seed_count: 2,
                network_seeds_ptr: 0x7300,
                network_seed_count: 1,
                accel_seeds_ptr: 0x8000,
                accel_seed_count: 1,
                bootstrap_payload: build_bootstrap_payload_descriptor(Some(
                    init_capsule.as_slice(),
                )),
                bootstrap_user_window: build_bootstrap_user_window_descriptor(
                    build_bootstrap_payload_descriptor(Some(init_capsule.as_slice())),
                ),
            },
        );
        plan.boot_info_v2 = Some(PayloadRegion {
            path: "bootinfo_v2",
            base: 0x7100,
            len: 128,
        });
        plan.stage1_image = Some(PayloadRegion {
            path: "stage1.bin",
            base: 0x8100,
            len: 4096,
        });
        plan.kernel_image = Some(PayloadRegion {
            path: "kernel.bin",
            base: 0x9100,
            len: 8192,
        });
        plan.loader_log = Some(PayloadRegion {
            path: "loader.log",
            base: 0xA100,
            len: 96,
        });

        let report = plan.structured_handoff_report(true);
        assert!(report.contains("report_kind=novaaa64_loader_handoff_report"));
        assert!(report.contains("stage1_plan_ready=true"));
        assert!(report.contains("boot_info_v2_valid=true"));
        assert!(report.contains("boot_info_v2_bootstrap_payload_present=true"));
        assert!(report.contains("boot_info_v2_bootstrap_user_window_present=true"));
        assert!(report.contains("boot_info_v2_bootstrap_user_window_base=0x40000000"));
        assert!(report.contains("boot_info_v2_bootstrap_user_window_size=131072"));
        assert!(report.contains("boot_info_v2_bootstrap_user_stack_size=32768"));
        assert!(report.contains("framebuffer_present=true"));
        assert!(report.contains("boot_info_v2.present=true"));
        assert!(report.contains("stage1_image.present=true"));
        assert!(report.contains("kernel_image.present=true"));
        assert!(report.contains("loader_log.present=true"));
    }

    fn build_init_capsule_with_payload() -> std::vec::Vec<u8> {
        let payload_body = [0x41u8, 0x42, 0x43, 0x44];
        let payload_header = NovaPayloadHeaderV1::new_flat_binary(
            NovaPayloadKind::Service,
            NovaPayloadEntryAbi::BootstrapTaskV1,
            (core::mem::size_of::<NovaPayloadHeaderV1>() + payload_body.len()) as u32,
            sha256_digest_bytes(&payload_body),
        );
        let mut payload =
            std::vec![0u8; core::mem::size_of::<NovaPayloadHeaderV1>() + payload_body.len()];
        payload[..core::mem::size_of::<NovaPayloadHeaderV1>()].copy_from_slice(unsafe {
            core::slice::from_raw_parts(
                &payload_header as *const NovaPayloadHeaderV1 as *const u8,
                core::mem::size_of::<NovaPayloadHeaderV1>(),
            )
        });
        payload[core::mem::size_of::<NovaPayloadHeaderV1>()..].copy_from_slice(&payload_body);

        let mut header = NovaInitCapsuleHeaderV1::new(
            encode_init_capsule_service_name("initd").expect("service name"),
            NovaInitCapsuleCapabilityV1::BootLog as u64,
            0,
            0,
        );
        header.total_size =
            (core::mem::size_of::<NovaInitCapsuleHeaderV1>() + payload.len()) as u32;

        let mut image = std::vec![0u8; header.total_size as usize];
        image[..core::mem::size_of::<NovaInitCapsuleHeaderV1>()].copy_from_slice(unsafe {
            core::slice::from_raw_parts(
                &header as *const NovaInitCapsuleHeaderV1 as *const u8,
                core::mem::size_of::<NovaInitCapsuleHeaderV1>(),
            )
        });
        image[core::mem::size_of::<NovaInitCapsuleHeaderV1>()..].copy_from_slice(&payload);
        image
    }
}

#[cfg(all(target_os = "uefi", target_arch = "aarch64"))]
fn sync_instruction_cache(ptr: *const u8, len: usize) {
    if ptr.is_null() || len == 0 {
        return;
    }

    let ctr_el0: u64;
    unsafe {
        core::arch::asm!("mrs {}, ctr_el0", out(reg) ctr_el0);
    }

    clean_data_cache(ptr, len);
    let icache_line = 4usize << (((ctr_el0 >> 16) & 0xf) as usize);
    let start = ptr as usize;
    let end = start + len;

    let mut line = start & !(icache_line - 1);
    while line < end {
        unsafe {
            core::arch::asm!("ic ivau, {}", in(reg) line);
        }
        line += icache_line;
    }

    unsafe {
        core::arch::asm!("dsb ish");
        core::arch::asm!("isb");
    }
}

#[cfg(not(all(target_os = "uefi", target_arch = "aarch64")))]
fn sync_instruction_cache(_ptr: *const u8, _len: usize) {}

#[cfg(all(target_os = "uefi", target_arch = "aarch64"))]
fn clean_data_cache(ptr: *const u8, len: usize) {
    if ptr.is_null() || len == 0 {
        return;
    }

    let ctr_el0: u64;
    unsafe {
        core::arch::asm!("mrs {}, ctr_el0", out(reg) ctr_el0);
    }

    let dcache_line = 4usize << ((ctr_el0 & 0xf) as usize);
    let start = ptr as usize;
    let end = start + len;

    let mut line = start & !(dcache_line - 1);
    while line < end {
        unsafe {
            core::arch::asm!("dc civac, {}", in(reg) line);
        }
        line += dcache_line;
    }

    unsafe {
        core::arch::asm!("dsb ish");
    }
}

#[cfg(not(all(target_os = "uefi", target_arch = "aarch64")))]
fn clean_data_cache(_ptr: *const u8, _len: usize) {}

#[cfg(all(target_os = "uefi", target_arch = "aarch64"))]
fn mask_interrupts() {
    unsafe {
        core::arch::asm!("msr daifset, #0xf", options(nostack, preserves_flags));
        core::arch::asm!("isb", options(nostack, preserves_flags));
    }
}

#[cfg(not(all(target_os = "uefi", target_arch = "aarch64")))]
fn mask_interrupts() {}

#[cfg(all(
    target_os = "uefi",
    target_arch = "aarch64",
    feature = "qemu_virt_trace"
))]
fn trace_post_exit_stage0() {
    qemu_uart_write(b"NovaOS stage0 post-exit\n");
}

#[cfg(not(all(
    target_os = "uefi",
    target_arch = "aarch64",
    feature = "qemu_virt_trace"
)))]
fn trace_post_exit_stage0() {}

#[cfg(all(
    target_os = "uefi",
    target_arch = "aarch64",
    feature = "qemu_virt_trace"
))]
fn qemu_uart_write(message: &[u8]) {
    const PL011_BASE: usize = 0x0900_0000;
    const PL011_DR: *mut u32 = PL011_BASE as *mut u32;
    const PL011_FR: *const u32 = (PL011_BASE + 0x18) as *const u32;
    const PL011_FR_TXFF: u32 = 1 << 5;

    for &byte in message {
        while unsafe { core::ptr::read_volatile(PL011_FR) } & PL011_FR_TXFF != 0 {}
        unsafe {
            core::ptr::write_volatile(PL011_DR, byte as u32);
        }
    }
}
