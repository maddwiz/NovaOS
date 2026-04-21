use crate::arch::arm64::allocator::FrameAllocatorPlan;
use crate::arch::arm64::exceptions::ExceptionVectors;
use crate::arch::arm64::mmu::PageTablePlan;
use crate::bootinfo::{
    self, BootSource, FramebufferFormat, NovaBootInfoV1, NovaBootInfoV2, NovaImageDigestV1,
    NovaVerificationInfoV1,
};
use crate::syscall::BootstrapTaskState;
use crate::trace_kernel_stage0_marker;
use core::mem::size_of;
use nova_rt::{
    NOVA_INIT_CAPSULE_SERVICE_NAME_LEN, NovaInitCapsuleHeaderV1, NovaPayloadEntryAbi,
    NovaPayloadKind, PayloadImage, decode_init_capsule_service_name,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KernelBringupV2State {
    pub cpu_arch: u16,
    pub platform_class: u16,
    pub memory_topology_class: u16,
    pub boot_source: BootSource,
    pub framebuffer_present: bool,
    pub storage_seed_count: u32,
    pub network_seed_count: u32,
    pub accel_seed_count: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BootstrapCapsuleSummary {
    pub service_name: [u8; NOVA_INIT_CAPSULE_SERVICE_NAME_LEN],
    pub requested_capabilities: u64,
    pub endpoint_slots: u32,
    pub shared_memory_regions: u32,
    pub payload_body_present: bool,
    pub payload_image_base: u64,
    pub payload_image_size: u64,
    pub payload_load_base: u64,
    pub payload_load_size: u64,
    pub payload_entry_point: u64,
    pub payload_descriptor_from_boot_info_v2: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BootstrapTaskLaunchPlan {
    pub service_name: [u8; NOVA_INIT_CAPSULE_SERVICE_NAME_LEN],
    pub image_base: u64,
    pub image_size: u64,
    pub load_base: u64,
    pub load_size: u64,
    pub entry_point: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BootstrapPayloadSource {
    BootInfoV2,
    InitCapsule,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ResolvedBootstrapPayload {
    image_base: u64,
    image_size: u64,
    load_base: u64,
    load_size: u64,
    entry_point: u64,
    source: BootstrapPayloadSource,
}

impl BootstrapTaskLaunchPlan {
    pub fn service_name(&self) -> &str {
        decode_init_capsule_service_name(&self.service_name)
            .expect("bootstrap task launch service name must stay valid")
    }
}

impl BootstrapCapsuleSummary {
    pub fn service_name(&self) -> &str {
        decode_init_capsule_service_name(&self.service_name)
            .expect("bootstrap capsule service name must stay valid")
    }

    pub const fn has_payload(&self) -> bool {
        self.payload_image_base != 0
            && self.payload_image_size != 0
            && self.payload_load_base != 0
            && self.payload_load_size != 0
            && self.payload_entry_point != 0
    }

    pub const fn task_state(&self) -> BootstrapTaskState {
        BootstrapTaskState::new(
            self.requested_capabilities,
            self.endpoint_slots,
            self.shared_memory_regions,
        )
    }

    pub const fn launch_plan(&self) -> Option<BootstrapTaskLaunchPlan> {
        if !self.has_payload() {
            return None;
        }

        Some(BootstrapTaskLaunchPlan {
            service_name: self.service_name,
            image_base: self.payload_image_base,
            image_size: self.payload_image_size,
            load_base: self.payload_load_base,
            load_size: self.payload_load_size,
            entry_point: self.payload_entry_point,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KernelBringupState {
    pub boot_summary: bootinfo::BootSummary,
    pub boot_info_v2: Option<KernelBringupV2State>,
    pub init_capsule: Option<BootstrapCapsuleSummary>,
    pub memory_map_bytes: usize,
    pub kernel_image_digest_present: bool,
    pub verification_info_present: bool,
    pub stage1_payload_verified: bool,
    pub kernel_payload_verified: bool,
    pub exception_vectors: ExceptionVectors,
    pub page_tables: PageTablePlan,
    pub allocator: FrameAllocatorPlan,
}

impl KernelBringupState {
    pub const fn empty() -> Self {
        Self {
            boot_summary: bootinfo::BootSummary::empty(),
            boot_info_v2: None,
            init_capsule: None,
            memory_map_bytes: 0,
            kernel_image_digest_present: false,
            verification_info_present: false,
            stage1_payload_verified: false,
            kernel_payload_verified: false,
            exception_vectors: ExceptionVectors::placeholder(),
            page_tables: PageTablePlan::empty(),
            allocator: FrameAllocatorPlan::empty(),
        }
    }
}

pub fn resolve_boot_info(boot_info: *const NovaBootInfoV1) -> Option<&'static NovaBootInfoV1> {
    let boot_info = unsafe { boot_info.as_ref() }?;
    boot_info.is_valid().then_some(boot_info)
}

pub fn resolve_boot_info_v2(
    boot_info_v2: *const NovaBootInfoV2,
) -> Option<&'static NovaBootInfoV2> {
    let boot_info_v2 = unsafe { boot_info_v2.as_ref() }?;
    boot_info_v2.is_valid().then_some(boot_info_v2)
}

pub fn resolve_optional_boot_info_v2(
    boot_info_v2: *const NovaBootInfoV2,
) -> Option<Option<&'static NovaBootInfoV2>> {
    if boot_info_v2.is_null() {
        return Some(None);
    }

    resolve_boot_info_v2(boot_info_v2).map(Some)
}

pub fn prepare_bringup(
    boot_info: &NovaBootInfoV1,
    boot_info_v2: Option<&NovaBootInfoV2>,
) -> Option<KernelBringupState> {
    let memory_map = match resolve_memory_map(boot_info) {
        Some(memory_map) => memory_map,
        None => {
            trace_kernel_stage0_marker(b"NovaOS kernel memory_map invalid\n");
            return None;
        }
    };
    let _descriptor_probe =
        memory_map.first().copied().unwrap_or(0) ^ memory_map.last().copied().unwrap_or(0);
    let verification = resolve_verification_info(boot_info);
    let kernel_digest_present = resolve_kernel_image_digest(boot_info)
        .map(NovaImageDigestV1::is_valid)
        .unwrap_or(false);
    let (boot_info_v2_summary, boot_info_v2) = match boot_info_v2 {
        Some(boot_info_v2) => match summarize_boot_info_v2(boot_info, boot_info_v2) {
            Some(summary) => (Some(summary), Some(boot_info_v2)),
            None => {
                trace_kernel_stage0_marker(b"NovaOS kernel bootinfo_v2 mismatch\n");
                return None;
            }
        },
        None => (None, None),
    };
    let init_capsule = match resolve_init_capsule(boot_info, boot_info_v2) {
        Some(init_capsule) => init_capsule,
        None => {
            trace_kernel_stage0_marker(b"NovaOS kernel init_capsule invalid\n");
            return None;
        }
    };

    boot_info.is_valid().then(|| KernelBringupState {
        boot_summary: boot_info.summary(),
        boot_info_v2: boot_info_v2_summary,
        init_capsule,
        memory_map_bytes: memory_map.len(),
        kernel_image_digest_present: kernel_digest_present,
        verification_info_present: verification.is_some(),
        stage1_payload_verified: verification
            .map(NovaVerificationInfoV1::stage1_payload_verified)
            .unwrap_or(false),
        kernel_payload_verified: verification
            .map(NovaVerificationInfoV1::kernel_payload_verified)
            .unwrap_or(false),
        exception_vectors: ExceptionVectors::runtime(),
        page_tables: PageTablePlan::from_boot_info_with_v2(boot_info, boot_info_v2),
        allocator: FrameAllocatorPlan::from_boot_info_with_v2(boot_info, boot_info_v2),
    })
}

fn resolve_init_capsule(
    boot_info: &NovaBootInfoV1,
    boot_info_v2: Option<&NovaBootInfoV2>,
) -> Option<Option<BootstrapCapsuleSummary>> {
    if boot_info.init_capsule_ptr == 0 && boot_info.init_capsule_len == 0 {
        return Some(None);
    }

    let ptr = boot_info.init_capsule_ptr as *const u8;
    let len = boot_info.init_capsule_len as usize;
    if ptr.is_null() || len == 0 {
        return None;
    }

    let bytes = unsafe { core::slice::from_raw_parts(ptr, len) };
    let header = parse_bootstrap_init_capsule_header(bytes)?;
    let body = &bytes[header.header_size as usize..];
    let payload = resolve_bootstrap_payload(body, boot_info_v2)?;
    let (
        payload_image_base,
        payload_image_size,
        payload_load_base,
        payload_load_size,
        payload_entry_point,
        payload_descriptor_from_boot_info_v2,
    ) = payload
        .map(|payload| {
            (
                payload.image_base,
                payload.image_size,
                payload.load_base,
                payload.load_size,
                payload.entry_point,
                matches!(payload.source, BootstrapPayloadSource::BootInfoV2),
            )
        })
        .unwrap_or((0, 0, 0, 0, 0, false));

    Some(Some(BootstrapCapsuleSummary {
        service_name: header.service_name,
        requested_capabilities: header.requested_capabilities,
        endpoint_slots: header.endpoint_slots,
        shared_memory_regions: header.shared_memory_regions,
        payload_body_present: !body.is_empty(),
        payload_image_base,
        payload_image_size,
        payload_load_base,
        payload_load_size,
        payload_entry_point,
        payload_descriptor_from_boot_info_v2,
    }))
}

fn resolve_bootstrap_payload(
    body: &[u8],
    boot_info_v2: Option<&NovaBootInfoV2>,
) -> Option<Option<ResolvedBootstrapPayload>> {
    if body.is_empty() {
        return Some(None);
    }

    if let Some(descriptor) = boot_info_v2
        .map(|boot_info_v2| boot_info_v2.bootstrap_payload)
        .filter(|descriptor| !descriptor.is_empty())
    {
        let body_base = body.as_ptr() as u64;
        if descriptor.image_ptr != body_base || descriptor.image_len != body.len() as u64 {
            trace_kernel_stage0_marker(b"NovaOS kernel bootstrap payload descriptor mismatch\n");
            return None;
        }

        return Some(Some(ResolvedBootstrapPayload {
            image_base: descriptor.image_ptr,
            image_size: descriptor.image_len,
            load_base: descriptor.load_base,
            load_size: descriptor.load_size,
            entry_point: descriptor.entry_point,
            source: BootstrapPayloadSource::BootInfoV2,
        }));
    }

    let image_base = body.as_ptr() as u64;
    Some(
        parse_bootstrap_init_payload(body).map(|payload| ResolvedBootstrapPayload {
            image_base,
            image_size: body.len() as u64,
            load_base: payload.load_base(image_base),
            load_size: payload.load_size(),
            entry_point: payload.entry_addr(image_base),
            source: BootstrapPayloadSource::InitCapsule,
        }),
    )
}

fn parse_bootstrap_init_capsule_header(bytes: &[u8]) -> Option<NovaInitCapsuleHeaderV1> {
    if bytes.len() < size_of::<NovaInitCapsuleHeaderV1>() {
        trace_kernel_stage0_marker(b"NovaOS kernel init_capsule short\n");
        return None;
    }

    let header = unsafe { (bytes.as_ptr() as *const NovaInitCapsuleHeaderV1).read_unaligned() };
    if !header.is_valid() {
        trace_kernel_stage0_marker(b"NovaOS kernel init_capsule header invalid\n");
        return None;
    }

    if !header.matches_image_len(bytes.len()) {
        trace_kernel_stage0_marker(b"NovaOS kernel init_capsule size mismatch\n");
        return None;
    }

    Some(header)
}

fn parse_bootstrap_init_payload(body: &[u8]) -> Option<PayloadImage<'_>> {
    if body.is_empty() {
        return None;
    }

    let payload = match PayloadImage::parse(body) {
        Some(payload) => payload,
        None => {
            trace_kernel_stage0_marker(b"NovaOS kernel bootstrap task image unavailable\n");
            return None;
        }
    };

    if payload.kind() != NovaPayloadKind::Service
        || payload.entry_abi() != NovaPayloadEntryAbi::BootstrapTaskV1
    {
        trace_kernel_stage0_marker(b"NovaOS kernel bootstrap task image unavailable\n");
        return None;
    }

    Some(payload)
}

fn summarize_boot_info_v2(
    boot_info: &NovaBootInfoV1,
    boot_info_v2: &NovaBootInfoV2,
) -> Option<KernelBringupV2State> {
    boot_info_v2_matches_v1(boot_info, boot_info_v2).then_some(KernelBringupV2State {
        cpu_arch: boot_info_v2.cpu_arch as u16,
        platform_class: boot_info_v2.platform_class as u16,
        memory_topology_class: boot_info_v2.memory_topology_class as u16,
        boot_source: boot_info_v2.boot_source,
        framebuffer_present: boot_info_v2.framebuffer_present(),
        storage_seed_count: boot_info_v2.storage_seed_count,
        network_seed_count: boot_info_v2.network_seed_count,
        accel_seed_count: boot_info_v2.accel_seed_count,
    })
}

fn boot_info_v2_matches_v1(boot_info: &NovaBootInfoV1, boot_info_v2: &NovaBootInfoV2) -> bool {
    boot_info.firmware_vendor_ptr == boot_info_v2.firmware_vendor_ptr
        && boot_info.firmware_revision == boot_info_v2.firmware_revision
        && boot_info.secure_boot_state == boot_info_v2.secure_boot_state
        && boot_info.boot_source == boot_info_v2.boot_source
        && boot_info.current_el == boot_info_v2.current_el
        && boot_info.memory_map_ptr == boot_info_v2.memory_map_ptr
        && boot_info.memory_map_entries == boot_info_v2.memory_map_entries
        && boot_info.memory_map_desc_size == boot_info_v2.memory_map_desc_size
        && boot_info.config_tables_ptr == boot_info_v2.config_tables_ptr
        && boot_info.config_table_count == boot_info_v2.config_table_count
        && boot_info.acpi_rsdp_ptr == boot_info_v2.acpi_rsdp_ptr
        && boot_info.dtb_ptr == boot_info_v2.dtb_ptr
        && boot_info.smbios_ptr == boot_info_v2.smbios_ptr
        && boot_info.init_capsule_ptr == boot_info_v2.init_capsule_ptr
        && boot_info.init_capsule_len == boot_info_v2.init_capsule_len
        && boot_info.loader_log_ptr == boot_info_v2.loader_log_ptr
        && boot_info.kernel_image_hash_ptr == boot_info_v2.kernel_image_hash_ptr
        && framebuffer_matches(boot_info, boot_info_v2)
}

fn framebuffer_matches(boot_info: &NovaBootInfoV1, boot_info_v2: &NovaBootInfoV2) -> bool {
    if boot_info.framebuffer_present() != boot_info_v2.framebuffer_present() {
        return false;
    }

    if !boot_info.framebuffer_present() {
        return true;
    }

    boot_info.framebuffer_base == boot_info_v2.framebuffer.base
        && boot_info.framebuffer_width == boot_info_v2.framebuffer.width
        && boot_info.framebuffer_height == boot_info_v2.framebuffer.height
        && boot_info.framebuffer_stride == boot_info_v2.framebuffer.stride
        && framebuffer_format_matches(
            boot_info.framebuffer_format,
            boot_info_v2.framebuffer.format,
        )
}

fn framebuffer_format_matches(lhs: FramebufferFormat, rhs: FramebufferFormat) -> bool {
    lhs == rhs
}

pub fn resolve_memory_map(boot_info: &NovaBootInfoV1) -> Option<&'static [u8]> {
    if !boot_info.is_valid() || !boot_info.memory_map_present() {
        return None;
    }

    let len = boot_info.memory_map_byte_len();
    let ptr = boot_info.memory_map_ptr as *const u8;

    (!ptr.is_null() && len != 0).then(|| unsafe { core::slice::from_raw_parts(ptr, len) })
}

pub fn resolve_kernel_image_digest(
    boot_info: &NovaBootInfoV1,
) -> Option<&'static NovaImageDigestV1> {
    if !boot_info.has_flag(NovaBootInfoV1::FLAG_HAS_KERNEL_IMAGE_DIGEST) {
        return None;
    }

    let ptr = boot_info.kernel_image_hash_ptr as *const NovaImageDigestV1;
    let digest = unsafe { ptr.as_ref() }?;
    digest.is_valid().then_some(digest)
}

pub fn resolve_verification_info(
    boot_info: &NovaBootInfoV1,
) -> Option<&'static NovaVerificationInfoV1> {
    if !boot_info.has_flag(NovaBootInfoV1::FLAG_HAS_VERIFICATION_INFO) {
        return None;
    }

    let ptr = boot_info.verification_info_ptr as *const NovaVerificationInfoV1;
    let verification = unsafe { ptr.as_ref() }?;
    verification.is_valid().then_some(verification)
}
