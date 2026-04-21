#![no_std]
#![cfg_attr(
    feature = "bootstrap_kernel_svc_probe",
    allow(dead_code, unreachable_code)
)]

#[cfg(test)]
extern crate alloc;

pub mod arch;
pub mod bootinfo;
pub mod console;
pub mod mm;
pub mod panic;
pub mod syscall;
pub mod trace;

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
use arch::arm64::allocator::BootstrapEl0BackingFramePlan;
use arch::arm64::allocator::FrameAllocatorPlan;
use arch::arm64::exceptions::ExceptionClass;
use arch::arm64::exceptions::ExceptionVectors;
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_kernel_svc_probe"
))]
use arch::arm64::exceptions::{
    BootstrapExceptionReturnCapture, read_bootstrap_exception_return_capture,
    reset_bootstrap_exception_return_capture,
};
use arch::arm64::mmu::PageTablePlan;
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
use arch::arm64::mmu::{
    BootstrapEl0MappingRequest, BootstrapEl0PageTableConstruction, BootstrapEl0PageTablePlan,
    construct_bootstrap_el0_page_tables,
};
use bootinfo::{
    BootSource, FramebufferFormat, NovaBootInfoV1, NovaBootInfoV2, NovaImageDigestV1,
    NovaVerificationInfoV1,
};
use console::{BootConsole, ConsoleSink};
use core::mem::size_of;

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
use crate::console::TraceConsole;
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
use nova_rt::NovaBootstrapTaskContextV2;
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    any(
        feature = "bootstrap_kernel_svc_probe",
        feature = "bootstrap_pretransfer_svc_probe"
    )
))]
use nova_rt::syscall::trace;
use nova_rt::{
    NOVA_INIT_CAPSULE_SERVICE_NAME_LEN, NovaBootstrapTaskContextV1, NovaInitCapsuleCapabilityV1,
    NovaInitCapsuleHeaderV1, NovaPayloadEntryAbi, NovaPayloadKind, NovaSyscallNumberV1,
    NovaSyscallRequestV1, NovaSyscallResultV1, NovaSyscallStatusV1, PayloadImage,
    decode_init_capsule_service_name, resolve_bootstrap_task_context,
};
use syscall::{
    Arm64SyscallFrame, BootstrapTaskState, CurrentTaskState, SyscallDispatchState,
    bootstrap_syscall_state, dispatch_syscall, handle_lower_el_bootstrap_syscall_exception,
    handle_syscall_exception, install_bootstrap_syscall_state,
};

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
type BootstrapTaskEntry = unsafe extern "C" fn(*const NovaBootstrapTaskContextV1) -> !;
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
const BOOTSTRAP_TASK_STACK_SIZE: usize = 64 * 1024;
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
const SPSR_EL2_MASKED_EL1H: usize = 0x3c5;
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_el0_probe"
))]
const SPSR_EL1_MASKED_EL0T: usize = 0x3c0;
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_el0_probe"
))]
const SCTLR_EL1_MMU_CACHE_ENABLE_MASK: usize = 0x1005;
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
const HCR_EL2_RW: usize = 1usize << 31;
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
static mut BOOTSTRAP_TASK_STACK: [u8; BOOTSTRAP_TASK_STACK_SIZE] = [0; BOOTSTRAP_TASK_STACK_SIZE];
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
static mut BOOTSTRAP_TASK_CONTEXT: NovaBootstrapTaskContextV2 = NovaBootstrapTaskContextV2::empty();

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_kernel_svc_probe"
))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
struct BootstrapKernelSvcCallerCapture {
    pre_x0: u64,
    pre_x1: u64,
    pre_x2: u64,
    post_x0: u64,
    post_x1: u64,
    post_x2: u64,
    valid: u64,
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_kernel_svc_probe"
))]
impl BootstrapKernelSvcCallerCapture {
    const VALID: u64 = 0x4B53_5643_4341_5054;

    const fn unset() -> Self {
        Self {
            pre_x0: u64::MAX,
            pre_x1: u64::MAX,
            pre_x2: u64::MAX,
            post_x0: u64::MAX,
            post_x1: u64::MAX,
            post_x2: u64::MAX,
            valid: 0,
        }
    }

    const fn is_recorded(self) -> bool {
        self.valid == Self::VALID
    }
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_kernel_svc_probe"
))]
static mut BOOTSTRAP_KERNEL_SVC_CALLER_CAPTURE: BootstrapKernelSvcCallerCapture =
    BootstrapKernelSvcCallerCapture::unset();

#[cfg(any(test, all(target_os = "none", target_arch = "aarch64")))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BootstrapTaskTransferMode {
    SameEl,
    DropToEl1,
    DropToEl0,
}

#[cfg(any(test, all(target_os = "none", target_arch = "aarch64")))]
impl BootstrapTaskTransferMode {
    const fn label(self) -> &'static str {
        match self {
            Self::SameEl => "same-el",
            Self::DropToEl1 => "drop-to-el1",
            Self::DropToEl0 => "drop-to-el0",
        }
    }
}

#[cfg(any(test, all(target_os = "none", target_arch = "aarch64")))]
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BootstrapTaskSyscallBoundary {
    CurrentElSvc,
    El0Svc,
}

#[cfg(any(test, all(target_os = "none", target_arch = "aarch64")))]
impl BootstrapTaskSyscallBoundary {
    const fn label(self) -> &'static str {
        match self {
            Self::CurrentElSvc => "current-el-svc",
            Self::El0Svc => "el0-svc",
        }
    }
}

#[cfg(any(test, all(target_os = "none", target_arch = "aarch64")))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BootstrapTaskBoundaryPlan {
    current_el: u8,
    target_el: u8,
    transfer_mode: BootstrapTaskTransferMode,
    task_isolated: bool,
    syscall_boundary: BootstrapTaskSyscallBoundary,
}

#[cfg(any(test, all(target_os = "none", target_arch = "aarch64")))]
const fn bootstrap_task_transfer_mode(current_el: u8) -> BootstrapTaskTransferMode {
    if current_el == 2 {
        BootstrapTaskTransferMode::DropToEl1
    } else {
        BootstrapTaskTransferMode::SameEl
    }
}

#[cfg(any(test, all(target_os = "none", target_arch = "aarch64")))]
const fn bootstrap_task_boundary_plan(current_el: u8) -> BootstrapTaskBoundaryPlan {
    let transfer_mode = bootstrap_task_transfer_mode(current_el);
    let target_el = match transfer_mode {
        BootstrapTaskTransferMode::DropToEl1 => 1,
        BootstrapTaskTransferMode::DropToEl0 => 0,
        BootstrapTaskTransferMode::SameEl => current_el,
    };

    BootstrapTaskBoundaryPlan {
        current_el,
        target_el,
        transfer_mode,
        task_isolated: false,
        syscall_boundary: BootstrapTaskSyscallBoundary::CurrentElSvc,
    }
}

#[cfg(any(test, all(target_os = "none", target_arch = "aarch64")))]
const fn bootstrap_task_target_boundary_plan(current_el: u8) -> BootstrapTaskBoundaryPlan {
    BootstrapTaskBoundaryPlan {
        current_el,
        target_el: 0,
        transfer_mode: BootstrapTaskTransferMode::DropToEl0,
        task_isolated: true,
        syscall_boundary: BootstrapTaskSyscallBoundary::El0Svc,
    }
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    any(
        feature = "bootstrap_kernel_svc_probe",
        feature = "bootstrap_pretransfer_svc_probe",
        feature = "bootstrap_trap_vector_trace"
    )
))]
const EXCEPTION_VECTOR_ALIGNMENT_MASK: u64 = 2048 - 1;

pub struct KernelContext<'a, C: ConsoleSink> {
    pub boot_info: &'a NovaBootInfoV1,
    pub boot_info_v2: Option<&'a NovaBootInfoV2>,
    pub bringup: Option<KernelBringupState>,
    pub console: &'a mut C,
}

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

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    any(
        feature = "bootstrap_kernel_svc_probe",
        feature = "bootstrap_pretransfer_svc_probe"
    )
))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RuntimeExceptionProbeState {
    current_el: u64,
    spsel: u64,
    vbar_el1: u64,
    expected_vbar_el1: u64,
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

pub fn kernel_main<C: ConsoleSink>(context: KernelContext<'_, C>) -> ! {
    context
        .console
        .log(console::LogLevel::Info, "NovaOS kernel bring-up");

    if !context.boot_info.is_valid() {
        context
            .console
            .log(console::LogLevel::Warn, "boot info marker is not set");
    }

    let summary = context
        .bringup
        .map(|bringup| bringup.boot_summary)
        .unwrap_or_else(|| context.boot_info.summary());
    context
        .console
        .log(console::LogLevel::Info, summary.describe());

    if context.boot_info_v2.is_some() {
        context
            .console
            .log(console::LogLevel::Info, "boot info v2 summary observed");
    }

    let bringup = context.bringup.unwrap_or_else(|| {
        prepare_bringup(context.boot_info, context.boot_info_v2)
            .unwrap_or_else(KernelBringupState::empty)
    });
    let vectors = bringup.exception_vectors;
    let _page_tables = bringup.page_tables;
    let _allocator = bringup.allocator;

    if let Some(init_capsule) = bringup.init_capsule {
        log_init_capsule_summary(context.console, init_capsule);
    }

    let bootstrap_syscall_state = bootstrap_syscall_dispatch_state(bringup.init_capsule);
    run_syscall_probe(context.console, bootstrap_syscall_state);
    install_bootstrap_exception_runtime(vectors, bootstrap_syscall_state);
    #[cfg(all(
        target_os = "none",
        target_arch = "aarch64",
        feature = "bootstrap_kernel_svc_probe"
    ))]
    run_bootstrap_kernel_svc_probe();

    #[cfg(not(all(
        target_os = "none",
        target_arch = "aarch64",
        feature = "bootstrap_kernel_svc_probe"
    )))]
    {
        let bootstrap_launch_plan = bringup
            .init_capsule
            .and_then(|init_capsule| init_capsule.launch_plan());
        if let Some(launch_plan) = bootstrap_launch_plan {
            context.console.write_str("[info] bootstrap task transfer ");
            context.console.write_line(launch_plan.service_name());
            enter_bootstrap_task(
                context.console,
                launch_plan,
                bringup.init_capsule,
                bringup.page_tables,
                bringup.allocator,
            );
        }
    }

    panic::log_and_halt(context.console, "kernel bring-up remains a scaffold");
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

pub fn kernel_entry(boot_info: *const NovaBootInfoV1) -> ! {
    let Some(boot_info) = resolve_boot_info(boot_info) else {
        panic::halt();
    };

    enter_kernel_runtime(boot_info, None, None)
}

pub fn kernel_stage0_entry(
    boot_info: *const NovaBootInfoV1,
    boot_info_v2: *const NovaBootInfoV2,
) -> ! {
    let Some(boot_info) = resolve_boot_info(boot_info) else {
        trace_kernel_stage0_marker(b"NovaOS kernel bootinfo invalid\n");
        panic::halt();
    };
    let Some(boot_info_v2) = resolve_optional_boot_info_v2(boot_info_v2) else {
        trace_kernel_stage0_marker(b"NovaOS kernel bootinfo_v2 invalid\n");
        panic::halt();
    };
    trace_kernel_stage0_marker(b"NovaOS kernel bootinfo_v2 ready\n");

    let Some(bringup) = prepare_bringup(boot_info, boot_info_v2) else {
        trace_kernel_stage0_marker(b"NovaOS kernel bringup invalid\n");
        panic::halt();
    };
    trace_kernel_stage0_marker(b"NovaOS kernel bringup ready\n");

    let _ = core::hint::black_box(bringup);
    enter_kernel_runtime(boot_info, boot_info_v2, Some(bringup))
}

pub fn kernel_identity() -> &'static str {
    "NovaOS kernel"
}

fn enter_kernel_runtime(
    boot_info: &'static NovaBootInfoV1,
    boot_info_v2: Option<&'static NovaBootInfoV2>,
    bringup: Option<KernelBringupState>,
) -> ! {
    let mut console = BootConsole::from_boot_info(boot_info);
    kernel_main(KernelContext {
        boot_info,
        boot_info_v2,
        bringup,
        console: &mut console,
    })
}

fn install_bootstrap_exception_runtime(
    vectors: ExceptionVectors,
    bootstrap_syscall_state: SyscallDispatchState,
) {
    install_bootstrap_syscall_state(bootstrap_syscall_state);
    let _installed_vectors = unsafe { vectors.install() };
    #[cfg(all(
        target_os = "none",
        target_arch = "aarch64",
        feature = "bootstrap_trap_vector_trace"
    ))]
    log_bootstrap_exception_install_status(vectors, _installed_vectors);
}

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
fn enter_bootstrap_task<C: ConsoleSink>(
    console: &mut C,
    launch_plan: BootstrapTaskLaunchPlan,
    init_capsule: Option<BootstrapCapsuleSummary>,
    page_tables: PageTablePlan,
    allocator: FrameAllocatorPlan,
) -> ! {
    sync_instruction_cache(
        launch_plan.image_base as *const u8,
        launch_plan.image_size as usize,
    );
    let context = init_capsule
        .map(build_bootstrap_task_context)
        .unwrap_or(core::ptr::null());
    log_bootstrap_el0_boundary_plan(console, launch_plan, context, page_tables, allocator);
    let payload_entry: BootstrapTaskEntry = unsafe {
        core::mem::transmute::<usize, BootstrapTaskEntry>(launch_plan.entry_point as usize)
    };
    let boundary_plan = bootstrap_task_boundary_plan(read_runtime_current_el());
    log_bootstrap_task_boundary(console, boundary_plan);
    let target_boundary_plan = bootstrap_task_target_boundary_plan(boundary_plan.current_el);
    log_bootstrap_task_target_boundary(console, target_boundary_plan);
    #[cfg(feature = "bootstrap_el0_probe")]
    let transfer_boundary_plan = target_boundary_plan;
    #[cfg(not(feature = "bootstrap_el0_probe"))]
    let transfer_boundary_plan = boundary_plan;
    #[cfg(feature = "bootstrap_pretransfer_svc_probe")]
    {
        let _ = payload_entry;
        unsafe {
            enter_bootstrap_task_with_stack(
                bootstrap_pretransfer_svc_probe_entry,
                context,
                transfer_boundary_plan,
            )
        }
    }
    #[cfg(not(feature = "bootstrap_pretransfer_svc_probe"))]
    unsafe {
        enter_bootstrap_task_with_stack(payload_entry, context, transfer_boundary_plan)
    }
}

#[cfg(not(all(target_os = "none", target_arch = "aarch64")))]
fn enter_bootstrap_task<C: ConsoleSink>(
    console: &mut C,
    _launch_plan: BootstrapTaskLaunchPlan,
    _init_capsule: Option<BootstrapCapsuleSummary>,
    _page_tables: PageTablePlan,
    _allocator: FrameAllocatorPlan,
) -> ! {
    panic::log_and_halt(
        console,
        "bootstrap task transfer is not supported on host builds",
    );
}

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
fn log_bootstrap_el0_boundary_plan<C: ConsoleSink>(
    console: &mut C,
    launch_plan: BootstrapTaskLaunchPlan,
    context: *const NovaBootstrapTaskContextV1,
    page_tables: PageTablePlan,
    allocator: FrameAllocatorPlan,
) {
    let context_size = if context.is_null() {
        0
    } else {
        size_of::<NovaBootstrapTaskContextV2>() as u64
    };
    let request = BootstrapEl0MappingRequest::new(
        launch_plan.load_base,
        launch_plan.load_size,
        launch_plan.entry_point,
        context as usize as u64,
        context_size,
        if page_tables.user_stack_size == 0 {
            (BOOTSTRAP_TASK_STACK_SIZE / 2) as u64
        } else {
            page_tables.user_stack_size
        },
    );
    let mapping = page_tables.bootstrap_el0_mapping_plan(request);

    console.write_str("[info] bootstrap el0 mapping ");
    console.write_line(mapping.readiness.label());

    let backing = BootstrapEl0BackingFramePlan::from_mapping_plan(
        mapping,
        allocator.bootstrap_el0_backing_frame_request(),
    );
    console.write_str("[info] bootstrap el0 backing frames ");
    console.write_line(backing.readiness.label());

    console.write_str("[info] bootstrap el0 page tables ");
    if backing.ready() {
        let page_table_plan = mapping.page_table_plan(
            backing.page_table_request(page_tables.kernel_base, page_tables.kernel_size),
        );
        console.write_line(page_table_plan.readiness.label());
        console.write_str("[info] bootstrap el0 page tables constructed ");
        if page_table_plan.ready() {
            let construction = unsafe { construct_live_bootstrap_el0_page_tables(page_table_plan) };
            console.write_line(construction.readiness.label());
        } else {
            console.write_line("page-tables-not-ready");
        }
    } else {
        console.write_line("backing-frames-not-ready");
        console.write_line("[info] bootstrap el0 page tables constructed page-tables-not-ready");
    }
}

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
unsafe fn construct_live_bootstrap_el0_page_tables(
    plan: BootstrapEl0PageTablePlan,
) -> BootstrapEl0PageTableConstruction {
    let entry_count = (plan.page_table_bytes / size_of::<u64>() as u64) as usize;
    let entries = unsafe {
        core::slice::from_raw_parts_mut(plan.page_table_phys_base as *mut u64, entry_count)
    };
    let construction = construct_bootstrap_el0_page_tables(plan, entries);
    if construction.ready() {
        clean_data_cache(
            plan.page_table_phys_base as *const u8,
            plan.page_table_bytes as usize,
        );
    }
    construction
}

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
fn build_bootstrap_task_context(
    init_capsule: BootstrapCapsuleSummary,
) -> *const NovaBootstrapTaskContextV1 {
    unsafe {
        BOOTSTRAP_TASK_CONTEXT = NovaBootstrapTaskContextV2::new(
            init_capsule.service_name,
            init_capsule.requested_capabilities,
            init_capsule.endpoint_slots,
            init_capsule.shared_memory_regions,
            novaos_bootstrap_kernel_call_v1 as *const () as usize as u64,
        );
        core::ptr::addr_of!(BOOTSTRAP_TASK_CONTEXT) as *const NovaBootstrapTaskContextV1
    }
}

#[allow(dead_code)]
fn dispatch_bootstrap_kernel_call<C: ConsoleSink>(
    context: *const NovaBootstrapTaskContextV1,
    request: NovaSyscallRequestV1,
    console: &mut C,
) -> NovaSyscallResultV1 {
    let Some(context) = resolve_bootstrap_task_context(context) else {
        return NovaSyscallResultV1::invalid_args();
    };
    let state = bootstrap_syscall_state();
    let Some(current_task) = state.current_task_service_name() else {
        return NovaSyscallResultV1::unsupported();
    };
    if current_task != context.service_name() {
        return NovaSyscallResultV1::denied();
    }

    console.write_str("[info] bootstrap kernel call from ");
    console.write_line(context.service_name());
    dispatch_syscall(&state, request, console)
}

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
#[unsafe(no_mangle)]
extern "C" fn novaos_bootstrap_kernel_call_v1(
    context: *const NovaBootstrapTaskContextV2,
    request: *const NovaSyscallRequestV1,
) -> NovaSyscallResultV1 {
    let Some(request) = (unsafe { request.as_ref() }).copied() else {
        return NovaSyscallResultV1::invalid_args();
    };

    let mut console = TraceConsole::new();
    dispatch_bootstrap_kernel_call(
        context as *const NovaBootstrapTaskContextV1,
        request,
        &mut console,
    )
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_kernel_svc_probe"
))]
fn bootstrap_trace_raw_registers(value0: u64, value1: u64) -> (u64, u64, u64) {
    let mut raw = [0u64; 3];

    unsafe {
        core::arch::asm!(
            "mov x0, x10",
            "mov x1, x11",
            "mov x2, xzr",
            "mov x3, xzr",
            "mov x4, xzr",
            "mov x5, xzr",
            "mov x6, xzr",
            "mov x7, xzr",
            "mov x8, x12",
            "svc #0",
            "stp x0, x1, [x9]",
            "str x2, [x9, #16]",
            in("x9") raw.as_mut_ptr(),
            in("x10") value0,
            in("x11") value1,
            in("x12") NovaSyscallNumberV1::Trace as u64,
            lateout("x0") _,
            lateout("x1") _,
            lateout("x2") _,
            lateout("x3") _,
            lateout("x4") _,
            lateout("x5") _,
            lateout("x6") _,
            lateout("x7") _,
            lateout("x8") _,
            options(nostack),
        );
    }

    (raw[0], raw[1], raw[2])
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_kernel_svc_probe"
))]
fn reset_bootstrap_kernel_svc_caller_capture() {
    let capture = core::ptr::addr_of_mut!(BOOTSTRAP_KERNEL_SVC_CALLER_CAPTURE);
    unsafe {
        core::ptr::write_volatile(capture, BootstrapKernelSvcCallerCapture::unset());
    }
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_kernel_svc_probe"
))]
fn read_bootstrap_kernel_svc_caller_capture_pre_x0() -> u64 {
    let capture = core::ptr::addr_of!(BOOTSTRAP_KERNEL_SVC_CALLER_CAPTURE);
    unsafe { core::ptr::read_volatile(core::ptr::addr_of!((*capture).pre_x0)) }
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_kernel_svc_probe"
))]
fn read_bootstrap_kernel_svc_caller_capture_pre_x1() -> u64 {
    let capture = core::ptr::addr_of!(BOOTSTRAP_KERNEL_SVC_CALLER_CAPTURE);
    unsafe { core::ptr::read_volatile(core::ptr::addr_of!((*capture).pre_x1)) }
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_kernel_svc_probe"
))]
fn read_bootstrap_kernel_svc_caller_capture_pre_x2() -> u64 {
    let capture = core::ptr::addr_of!(BOOTSTRAP_KERNEL_SVC_CALLER_CAPTURE);
    unsafe { core::ptr::read_volatile(core::ptr::addr_of!((*capture).pre_x2)) }
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_kernel_svc_probe"
))]
fn read_bootstrap_kernel_svc_caller_capture_post_x0() -> u64 {
    let capture = core::ptr::addr_of!(BOOTSTRAP_KERNEL_SVC_CALLER_CAPTURE);
    unsafe { core::ptr::read_volatile(core::ptr::addr_of!((*capture).post_x0)) }
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_kernel_svc_probe"
))]
fn read_bootstrap_kernel_svc_caller_capture_post_x1() -> u64 {
    let capture = core::ptr::addr_of!(BOOTSTRAP_KERNEL_SVC_CALLER_CAPTURE);
    unsafe { core::ptr::read_volatile(core::ptr::addr_of!((*capture).post_x1)) }
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_kernel_svc_probe"
))]
fn read_bootstrap_kernel_svc_caller_capture_post_x2() -> u64 {
    let capture = core::ptr::addr_of!(BOOTSTRAP_KERNEL_SVC_CALLER_CAPTURE);
    unsafe { core::ptr::read_volatile(core::ptr::addr_of!((*capture).post_x2)) }
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_kernel_svc_probe"
))]
fn read_bootstrap_kernel_svc_caller_capture_valid() -> u64 {
    let capture = core::ptr::addr_of!(BOOTSTRAP_KERNEL_SVC_CALLER_CAPTURE);
    unsafe { core::ptr::read_volatile(core::ptr::addr_of!((*capture).valid)) }
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_kernel_svc_probe"
))]
fn bootstrap_trace_capture_caller_registers(value0: u64, value1: u64) {
    let capture = core::ptr::addr_of_mut!(BOOTSTRAP_KERNEL_SVC_CALLER_CAPTURE);

    unsafe {
        core::arch::asm!(
            "mov x0, x10",
            "mov x1, x11",
            "mov x2, xzr",
            "mov x3, xzr",
            "mov x4, xzr",
            "mov x5, xzr",
            "mov x6, xzr",
            "mov x7, xzr",
            "mov x8, x12",
            "stp x0, x1, [x9]",
            "str x2, [x9, #16]",
            "svc #0",
            "stp x0, x1, [x9, #24]",
            "str x2, [x9, #40]",
            "mov x3, x13",
            "str x3, [x9, #48]",
            in("x9") capture,
            in("x10") value0,
            in("x11") value1,
            in("x12") NovaSyscallNumberV1::Trace as u64,
            in("x13") BootstrapKernelSvcCallerCapture::VALID,
            lateout("x0") _,
            lateout("x1") _,
            lateout("x2") _,
            lateout("x3") _,
            lateout("x4") _,
            lateout("x5") _,
            lateout("x6") _,
            lateout("x7") _,
            lateout("x8") _,
            options(nostack),
        );
    }
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_kernel_svc_probe"
))]
fn run_bootstrap_kernel_svc_probe() -> ! {
    const TRACE_VALUE0: u64 = 0x4B45_5253_5643_3031;
    const TRACE_VALUE1: u64 = 0x4E4F_5641_4B45_524E;

    log_runtime_exception_probe_state(
        "bootstrap kernel svc runtime",
        read_runtime_exception_probe_state(),
    );
    trace_kernel_stage0_marker(b"NovaOS bootstrap kernel svc begin\n");
    reset_bootstrap_kernel_svc_caller_capture();
    bootstrap_trace_capture_caller_registers(TRACE_VALUE0, TRACE_VALUE1);
    let caller_pre_x0 = read_bootstrap_kernel_svc_caller_capture_pre_x0();
    let caller_pre_x1 = read_bootstrap_kernel_svc_caller_capture_pre_x1();
    let caller_pre_x2 = read_bootstrap_kernel_svc_caller_capture_pre_x2();
    let caller_post_x0 = read_bootstrap_kernel_svc_caller_capture_post_x0();
    let caller_post_x1 = read_bootstrap_kernel_svc_caller_capture_post_x1();
    let caller_post_x2 = read_bootstrap_kernel_svc_caller_capture_post_x2();
    let caller_valid = read_bootstrap_kernel_svc_caller_capture_valid();
    reset_bootstrap_exception_return_capture();
    let (raw_x0, raw_x1, raw_x2) = bootstrap_trace_raw_registers(TRACE_VALUE0, TRACE_VALUE1);
    let return_capture = read_bootstrap_exception_return_capture();
    let mut console = TraceConsole::new();
    log_bootstrap_kernel_svc_caller_capture(
        &mut console,
        caller_pre_x0,
        caller_pre_x1,
        caller_pre_x2,
        caller_post_x0,
        caller_post_x1,
        caller_post_x2,
        caller_valid,
    );
    console.write_str("[info] bootstrap kernel svc raw x0 ");
    write_hex_u64(&mut console, raw_x0);
    console.write_str(" x1 ");
    write_hex_u64(&mut console, raw_x1);
    console.write_str(" x2 ");
    write_hex_u64(&mut console, raw_x2);
    console.write_str("\n");
    log_bootstrap_exception_return_capture(&mut console, return_capture);
    if caller_valid == BootstrapKernelSvcCallerCapture::VALID
        && caller_post_x0 == NovaSyscallStatusV1::Ok as u64
        && caller_post_x1 == TRACE_VALUE0
        && caller_post_x2 == TRACE_VALUE1
    {
        trace_kernel_stage0_marker(b"NovaOS bootstrap kernel svc caller capture matched\n");
    } else {
        trace_kernel_stage0_marker(b"NovaOS bootstrap kernel svc caller capture mismatch\n");
    }
    let result = trace(TRACE_VALUE0, TRACE_VALUE1);
    if result.status == NovaSyscallStatusV1::Ok as u32
        && result.value0 == TRACE_VALUE0
        && result.value1 == TRACE_VALUE1
    {
        trace_kernel_stage0_marker(b"NovaOS bootstrap kernel svc passed\n");
    } else {
        let mut console = TraceConsole::new();
        if result.status != NovaSyscallStatusV1::Ok as u32 {
            trace_kernel_stage0_marker(b"NovaOS bootstrap kernel svc status mismatch\n");
        }
        if result.value0 != TRACE_VALUE0 {
            trace_kernel_stage0_marker(b"NovaOS bootstrap kernel svc value0 mismatch\n");
        }
        if result.value1 != TRACE_VALUE1 {
            trace_kernel_stage0_marker(b"NovaOS bootstrap kernel svc value1 mismatch\n");
        }
        console.write_str("[info] bootstrap kernel svc result status ");
        write_hex_u64(&mut console, result.status as u64);
        console.write_str(" value0 ");
        write_hex_u64(&mut console, result.value0);
        console.write_str(" value1 ");
        write_hex_u64(&mut console, result.value1);
        console.write_str("\n");
        trace_kernel_stage0_marker(b"NovaOS bootstrap kernel svc failed\n");
    }

    panic::halt();
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_kernel_svc_probe"
))]
fn log_bootstrap_exception_return_capture(
    console: &mut TraceConsole,
    capture: BootstrapExceptionReturnCapture,
) {
    if !capture.is_recorded() {
        console.write_line("[info] bootstrap kernel svc capture unavailable");
        return;
    }

    console.write_str("[info] bootstrap kernel svc frame x0 ");
    write_hex_u64(console, capture.frame_x0);
    console.write_str(" x1 ");
    write_hex_u64(console, capture.frame_x1);
    console.write_str(" x2 ");
    write_hex_u64(console, capture.frame_x2);
    console.write_str("\n");

    console.write_str("[info] bootstrap kernel svc restore x0 ");
    write_hex_u64(console, capture.restored_x0);
    console.write_str(" x1 ");
    write_hex_u64(console, capture.restored_x1);
    console.write_str(" x2 ");
    write_hex_u64(console, capture.restored_x2);
    console.write_str("\n");
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_kernel_svc_probe"
))]
fn log_bootstrap_kernel_svc_caller_capture(
    console: &mut TraceConsole,
    pre_x0: u64,
    pre_x1: u64,
    pre_x2: u64,
    post_x0: u64,
    post_x1: u64,
    post_x2: u64,
    valid: u64,
) {
    if valid != BootstrapKernelSvcCallerCapture::VALID {
        console.write_line("[info] bootstrap kernel svc caller capture unavailable");
        return;
    }

    console.write_str("[info] bootstrap kernel svc caller pre x0 ");
    write_hex_u64(console, pre_x0);
    console.write_str(" x1 ");
    write_hex_u64(console, pre_x1);
    console.write_str(" x2 ");
    write_hex_u64(console, pre_x2);
    console.write_str("\n");

    console.write_str("[info] bootstrap kernel svc caller post x0 ");
    write_hex_u64(console, post_x0);
    console.write_str(" x1 ");
    write_hex_u64(console, post_x1);
    console.write_str(" x2 ");
    write_hex_u64(console, post_x2);
    console.write_str("\n");
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_pretransfer_svc_probe"
))]
unsafe extern "C" fn bootstrap_pretransfer_svc_probe_entry(
    context: *const NovaBootstrapTaskContextV1,
) -> ! {
    const TRACE_VALUE0: u64 = 0x5052_4553_5643_3031;
    const TRACE_VALUE1: u64 = 0x4E4F_5641_5052_4554;

    if resolve_bootstrap_task_context(context).is_none() {
        trace_kernel_stage0_marker(b"NovaOS bootstrap pretransfer svc invalid context\n");
        panic::halt();
    }

    log_runtime_exception_probe_state(
        "bootstrap pretransfer svc runtime",
        read_runtime_exception_probe_state(),
    );
    trace_kernel_stage0_marker(b"NovaOS bootstrap pretransfer svc begin\n");
    let result = trace(TRACE_VALUE0, TRACE_VALUE1);
    if result.status == NovaSyscallStatusV1::Ok as u32
        && result.value0 == TRACE_VALUE0
        && result.value1 == TRACE_VALUE1
    {
        trace_kernel_stage0_marker(b"NovaOS bootstrap pretransfer svc passed\n");
    } else {
        trace_kernel_stage0_marker(b"NovaOS bootstrap pretransfer svc failed\n");
    }

    panic::halt();
}

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
unsafe fn enter_bootstrap_task_with_stack(
    entry: BootstrapTaskEntry,
    context: *const NovaBootstrapTaskContextV1,
    boundary_plan: BootstrapTaskBoundaryPlan,
) -> ! {
    let el1_stack_top = unsafe {
        let stack_base = core::ptr::addr_of_mut!(BOOTSTRAP_TASK_STACK) as *mut u8;
        (stack_base.add(BOOTSTRAP_TASK_STACK_SIZE) as usize) & !0xfusize
    };
    #[cfg(feature = "bootstrap_el0_probe")]
    let el0_stack_top = unsafe {
        let stack_base = core::ptr::addr_of_mut!(BOOTSTRAP_TASK_STACK) as *mut u8;
        (stack_base.add(BOOTSTRAP_TASK_STACK_SIZE / 2) as usize) & !0xfusize
    };
    match boundary_plan.transfer_mode {
        BootstrapTaskTransferMode::SameEl => unsafe {
            enter_bootstrap_task_same_el(entry, context, el1_stack_top)
        },
        BootstrapTaskTransferMode::DropToEl1 => unsafe {
            enter_bootstrap_task_via_el1(entry, context, el1_stack_top)
        },
        #[cfg(feature = "bootstrap_el0_probe")]
        BootstrapTaskTransferMode::DropToEl0 => unsafe {
            enter_bootstrap_task_via_el0(entry, context, el0_stack_top, el1_stack_top)
        },
        #[cfg(not(feature = "bootstrap_el0_probe"))]
        BootstrapTaskTransferMode::DropToEl0 => panic::halt(),
    }
}

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
fn log_bootstrap_task_boundary<C: ConsoleSink>(
    console: &mut C,
    boundary_plan: BootstrapTaskBoundaryPlan,
) {
    console.write_str("[info] bootstrap task boundary ");
    console.write_line(boundary_plan.transfer_mode.label());
    console.write_str("[info] bootstrap task boundary current_el ");
    write_hex_u64(console, boundary_plan.current_el as u64);
    console.write_str(" target_el ");
    write_hex_u64(console, boundary_plan.target_el as u64);
    console.write_str(" isolated ");
    if boundary_plan.task_isolated {
        console.write_str("true");
    } else {
        console.write_str("false");
    }
    console.write_str(" syscall ");
    console.write_line(boundary_plan.syscall_boundary.label());
}

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
fn log_bootstrap_task_target_boundary<C: ConsoleSink>(
    console: &mut C,
    boundary_plan: BootstrapTaskBoundaryPlan,
) {
    console.write_str("[info] bootstrap task target boundary ");
    console.write_line(boundary_plan.transfer_mode.label());
    console.write_str("[info] bootstrap task target boundary current_el ");
    write_hex_u64(console, boundary_plan.current_el as u64);
    console.write_str(" target_el ");
    write_hex_u64(console, boundary_plan.target_el as u64);
    console.write_str(" isolated ");
    if boundary_plan.task_isolated {
        console.write_str("true");
    } else {
        console.write_str("false");
    }
    console.write_str(" syscall ");
    console.write_line(boundary_plan.syscall_boundary.label());
}

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
fn read_runtime_current_el() -> u8 {
    let current_el: u64;
    unsafe {
        core::arch::asm!("mrs {}, CurrentEL", out(reg) current_el);
    }
    ((current_el >> 2) & 0b11) as u8
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    any(
        feature = "bootstrap_kernel_svc_probe",
        feature = "bootstrap_pretransfer_svc_probe",
        feature = "bootstrap_trap_vector_trace"
    )
))]
fn read_runtime_vbar_el1() -> u64 {
    let vbar_el1: u64;
    unsafe {
        core::arch::asm!("mrs {}, vbar_el1", out(reg) vbar_el1);
    }
    vbar_el1
}

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
unsafe fn enter_bootstrap_task_same_el(
    entry: BootstrapTaskEntry,
    context: *const NovaBootstrapTaskContextV1,
    stack_top: usize,
) -> ! {
    unsafe {
        core::arch::asm!(
            "msr SPSel, #1",
            "isb",
            "msr sp_el0, x10",
            "mov sp, x10",
            "mov x1, xzr",
            "mov x2, xzr",
            "mov x3, xzr",
            "mov x4, xzr",
            "mov x5, xzr",
            "mov x6, xzr",
            "mov x7, xzr",
            "mov x8, xzr",
            "mov x29, xzr",
            "mov x30, xzr",
            "br x9",
            in("x0") context as usize,
            in("x9") entry as usize,
            in("x10") stack_top,
            options(noreturn),
        );
    }
}

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
unsafe fn enter_bootstrap_task_via_el1(
    entry: BootstrapTaskEntry,
    context: *const NovaBootstrapTaskContextV1,
    stack_top: usize,
) -> ! {
    unsafe {
        core::arch::asm!(
            "mrs x13, hcr_el2",
            "orr x13, x13, x12",
            "msr hcr_el2, x13",
            "isb",
            "msr sp_el0, x10",
            "msr sp_el1, x10",
            "msr elr_el2, x9",
            "msr spsr_el2, x11",
            "mov x1, xzr",
            "mov x2, xzr",
            "mov x3, xzr",
            "mov x4, xzr",
            "mov x5, xzr",
            "mov x6, xzr",
            "mov x7, xzr",
            "mov x8, xzr",
            "mov x29, xzr",
            "mov x30, xzr",
            "eret",
            in("x0") context as usize,
            in("x9") entry as usize,
            in("x10") stack_top,
            in("x11") SPSR_EL2_MASKED_EL1H,
            in("x12") HCR_EL2_RW,
            options(noreturn),
        );
    }
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_el0_probe"
))]
unsafe fn enter_bootstrap_task_via_el0(
    entry: BootstrapTaskEntry,
    context: *const NovaBootstrapTaskContextV1,
    el0_stack_top: usize,
    el1_stack_top: usize,
) -> ! {
    unsafe {
        core::arch::asm!(
            "msr SPSel, #1",
            "isb",
            "mov sp, x12",
            "msr sp_el0, x10",
            "msr elr_el1, x9",
            "msr spsr_el1, x11",
            "mrs x13, sctlr_el1",
            "bic x13, x13, x14",
            "dsb sy",
            "msr sctlr_el1, x13",
            "isb",
            "mov x1, xzr",
            "mov x2, xzr",
            "mov x3, xzr",
            "mov x4, xzr",
            "mov x5, xzr",
            "mov x6, xzr",
            "mov x7, xzr",
            "mov x8, xzr",
            "mov x29, xzr",
            "mov x30, xzr",
            "eret",
            in("x0") context as usize,
            in("x9") entry as usize,
            in("x10") el0_stack_top,
            in("x11") SPSR_EL1_MASKED_EL0T,
            in("x12") el1_stack_top,
            in("x14") SCTLR_EL1_MMU_CACHE_ENABLE_MASK,
            options(noreturn),
        );
    }
}

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
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

#[cfg(not(all(target_os = "none", target_arch = "aarch64")))]
#[allow(dead_code)]
fn sync_instruction_cache(_ptr: *const u8, _len: usize) {}

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
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
            core::arch::asm!("dc cvau, {}", in(reg) line);
        }
        line += dcache_line;
    }

    unsafe {
        core::arch::asm!("dsb ish");
    }
}

#[cfg(not(all(target_os = "none", target_arch = "aarch64")))]
#[allow(dead_code)]
fn clean_data_cache(_ptr: *const u8, _len: usize) {}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    any(
        feature = "bootstrap_kernel_svc_probe",
        feature = "bootstrap_pretransfer_svc_probe"
    )
))]
fn read_runtime_exception_probe_state() -> RuntimeExceptionProbeState {
    let current_el: u64;
    let spsel: u64;
    unsafe {
        core::arch::asm!("mrs {}, CurrentEL", out(reg) current_el);
        core::arch::asm!("mrs {}, SPSel", out(reg) spsel);
    }

    RuntimeExceptionProbeState {
        current_el: (current_el >> 2) & 0b11,
        spsel,
        vbar_el1: read_runtime_vbar_el1(),
        expected_vbar_el1: ExceptionVectors::installed_or_runtime().base,
    }
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    any(
        feature = "bootstrap_kernel_svc_probe",
        feature = "bootstrap_pretransfer_svc_probe"
    )
))]
fn log_runtime_exception_probe_state(label: &str, state: RuntimeExceptionProbeState) {
    let mut console = TraceConsole::new();
    console.write_str("[info] ");
    console.write_str(label);
    console.write_str(" current_el_is_el1 ");
    if state.current_el == 1 {
        console.write_line("true");
    } else {
        console.write_line("false");
    }

    console.write_str("[info] ");
    console.write_str(label);
    console.write_str(" spsel_is_spx ");
    if state.spsel == 1 {
        console.write_line("true");
    } else {
        console.write_line("false");
    }

    console.write_str("[info] ");
    console.write_str(label);
    console.write_str(" runtime_vbar_aligned ");
    if (state.expected_vbar_el1 & EXCEPTION_VECTOR_ALIGNMENT_MASK) == 0 {
        console.write_line("true");
    } else {
        console.write_line("false");
    }

    console.write_str("[info] ");
    console.write_str(label);
    console.write_str(" vbar_matches_runtime ");
    if state.vbar_el1 == state.expected_vbar_el1 {
        console.write_line("true");
    } else {
        console.write_line("false");
    }
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_trap_vector_trace"
))]
fn log_bootstrap_exception_install_status(
    vectors: ExceptionVectors,
    installed_vectors: ExceptionVectors,
) {
    let readback_vbar = read_runtime_vbar_el1();

    if (vectors.base & EXCEPTION_VECTOR_ALIGNMENT_MASK) == 0 {
        trace_kernel_stage0_marker(b"NovaOS bootstrap vector base aligned\n");
    } else {
        trace_kernel_stage0_marker(b"NovaOS bootstrap vector base misaligned\n");
    }

    if readback_vbar == installed_vectors.base {
        trace_kernel_stage0_marker(b"NovaOS bootstrap vbar install match\n");
    } else {
        trace_kernel_stage0_marker(b"NovaOS bootstrap vbar install mismatch\n");
    }
}

fn log_init_capsule_summary<C: ConsoleSink>(
    console: &mut C,
    init_capsule: BootstrapCapsuleSummary,
) {
    console.log(console::LogLevel::Info, "init capsule summary observed");
    console.write_str("[info] init capsule service ");
    console.write_line(init_capsule.service_name());
    console.write_str("[info] bootstrap task current ");
    console.write_line(init_capsule.service_name());
    if init_capsule.payload_body_present {
        console.log(console::LogLevel::Info, "bootstrap task image observed");
    }
    if let Some(launch_plan) = init_capsule.launch_plan() {
        let _ = launch_plan;
        if init_capsule.payload_descriptor_from_boot_info_v2 {
            console.log(
                console::LogLevel::Info,
                "bootstrap task launch plan from bootinfo_v2",
            );
        } else if init_capsule.payload_body_present {
            console.log(console::LogLevel::Info, "bootstrap task image staged");
        }
    } else if init_capsule.payload_body_present {
        console.log(console::LogLevel::Info, "bootstrap task image staged");
    }
}

fn bootstrap_syscall_dispatch_state(
    init_capsule: Option<BootstrapCapsuleSummary>,
) -> SyscallDispatchState {
    init_capsule
        .map(|init_capsule| {
            let task_state = init_capsule.task_state();
            let endpoints_ready = task_state
                .has_bootstrap_capability(NovaInitCapsuleCapabilityV1::EndpointBootstrap)
                && task_state.endpoint_slots != 0;
            let shared_memory_ready = task_state
                .has_bootstrap_capability(NovaInitCapsuleCapabilityV1::SharedMemoryBootstrap)
                && task_state.shared_memory_regions != 0;
            SyscallDispatchState::bootstrap(
                CurrentTaskState::new(init_capsule.service_name, task_state),
                endpoints_ready,
                shared_memory_ready,
            )
        })
        .unwrap_or_else(SyscallDispatchState::scaffold)
}

fn run_syscall_probe<C: ConsoleSink>(console: &mut C, state: SyscallDispatchState) {
    let denied_trace = dispatch_syscall(
        &SyscallDispatchState::scaffold(),
        NovaSyscallRequestV1::new(NovaSyscallNumberV1::Trace, 0, [0xDEAD_BEEF, 0, 0, 0, 0, 0]),
        console,
    );
    let request = NovaSyscallRequestV1::new(
        NovaSyscallNumberV1::Trace,
        0,
        [0xCAFE_BABE, 0x5151_0001, 0, 0, 0, 0],
    );
    let mut frame = Arm64SyscallFrame::from_request(request);
    frame.elr = 0x4000;

    let handled = handle_syscall_exception(
        (ExceptionClass::Svc64 as u32) << 26,
        &mut frame,
        &state,
        console,
    );

    if denied_trace.status == NovaSyscallStatusV1::Denied as u32
        && handled
        && frame.registers[Arm64SyscallFrame::STATUS_REGISTER] == NovaSyscallStatusV1::Ok as u64
        && frame.registers[Arm64SyscallFrame::VALUE0_REGISTER] == 0xCAFE_BABE
        && frame.registers[Arm64SyscallFrame::VALUE1_REGISTER] == 0x5151_0001
        && frame.elr == 0x4004
    {
        console.log(console::LogLevel::Info, "bootstrap capability probe passed");
    } else {
        console.log(
            console::LogLevel::Error,
            "bootstrap capability probe failed",
        );
    }

    let endpoint_probe_result = (state
        .has_bootstrap_capability(NovaInitCapsuleCapabilityV1::EndpointBootstrap)
        && state.contains_endpoint_slot(0))
    .then(|| {
        dispatch_syscall(
            &state,
            NovaSyscallRequestV1::new(
                NovaSyscallNumberV1::EndpointCall,
                0,
                [0, 0x454E_4450, 0, 0, 0, 0],
            ),
            console,
        )
    });

    match endpoint_probe_result {
        Some(result) => {
            let status_raw = result.status;
            if status_raw == NovaSyscallStatusV1::Ok as u32
                && result.value0 == 0
                && result.value1 == 0x454E_4450
            {
                console.log(console::LogLevel::Info, "bootstrap endpoint probe passed");
            } else {
                console.log(console::LogLevel::Error, "bootstrap endpoint probe failed");
                console.write_str("[error] bootstrap endpoint probe status ");
                write_hex_u64(console, status_raw as u64);
                console.write_str(" value0 ");
                write_hex_u64(console, result.value0);
                console.write_str(" value1 ");
                write_hex_u64(console, result.value1);
                console.write_str("\n");
            }
        }
        None => console.log(console::LogLevel::Info, "bootstrap endpoint probe skipped"),
    }

    let shared_memory_probe_result = (state
        .has_bootstrap_capability(NovaInitCapsuleCapabilityV1::SharedMemoryBootstrap)
        && state.contains_shared_memory_region(0))
    .then(|| {
        dispatch_syscall(
            &state,
            NovaSyscallRequestV1::new(
                NovaSyscallNumberV1::SharedMemoryMap,
                0,
                [0, 0x5348_4D45_4D30, 0, 0, 0, 0],
            ),
            console,
        )
    });

    match shared_memory_probe_result {
        Some(result) => {
            let status_raw = result.status;
            if status_raw == NovaSyscallStatusV1::Ok as u32
                && result.value0 == 0
                && result.value1 == 0x5348_4D45_4D30
            {
                console.log(
                    console::LogLevel::Info,
                    "bootstrap shared memory probe passed",
                );
            } else {
                console.log(
                    console::LogLevel::Error,
                    "bootstrap shared memory probe failed",
                );
                console.write_str("[error] bootstrap shared memory probe status ");
                write_hex_u64(console, status_raw as u64);
                console.write_str(" value0 ");
                write_hex_u64(console, result.value0);
                console.write_str(" value1 ");
                write_hex_u64(console, result.value1);
                console.write_str("\n");
            }
        }
        None => console.log(
            console::LogLevel::Info,
            "bootstrap shared memory probe skipped",
        ),
    }

    run_lower_el_bootstrap_svc_dry_run(console, &state);
}

fn run_lower_el_bootstrap_svc_dry_run<C: ConsoleSink>(
    console: &mut C,
    state: &SyscallDispatchState,
) {
    const TRACE_VALUE0: u64 = 0x4C4F_5745_4C53_5643;
    const TRACE_VALUE1: u64 = 0x4E4F_5641_454C_3030;
    const RETURN_ELR: u64 = 0x8004;

    let request = NovaSyscallRequestV1::new(
        NovaSyscallNumberV1::Trace,
        0,
        [TRACE_VALUE0, TRACE_VALUE1, 0, 0, 0, 0],
    );
    let mut frame = Arm64SyscallFrame::from_request(request);
    frame.elr = RETURN_ELR - Arm64SyscallFrame::SYSCALL_INSTRUCTION_LEN;

    install_bootstrap_syscall_state(*state);
    let handled = handle_lower_el_bootstrap_syscall_exception(
        (ExceptionClass::Svc64 as u32) << 26,
        &mut frame,
        console,
    );

    if handled
        && frame.registers[Arm64SyscallFrame::STATUS_REGISTER] == NovaSyscallStatusV1::Ok as u64
        && frame.registers[Arm64SyscallFrame::VALUE0_REGISTER] == TRACE_VALUE0
        && frame.registers[Arm64SyscallFrame::VALUE1_REGISTER] == TRACE_VALUE1
        && frame.elr == RETURN_ELR
    {
        console.log(
            console::LogLevel::Info,
            "bootstrap lower-el svc dry-run passed",
        );
    } else {
        console.log(
            console::LogLevel::Error,
            "bootstrap lower-el svc dry-run failed",
        );
        console.write_str("[error] bootstrap lower-el svc dry-run handled ");
        if handled {
            console.write_str("true");
        } else {
            console.write_str("false");
        }
        console.write_str(" status ");
        write_hex_u64(console, frame.registers[Arm64SyscallFrame::STATUS_REGISTER]);
        console.write_str(" value0 ");
        write_hex_u64(console, frame.registers[Arm64SyscallFrame::VALUE0_REGISTER]);
        console.write_str(" value1 ");
        write_hex_u64(console, frame.registers[Arm64SyscallFrame::VALUE1_REGISTER]);
        console.write_str(" elr ");
        write_hex_u64(console, frame.elr);
        console.write_str("\n");
    }
}

fn write_hex_u64<C: ConsoleSink>(console: &mut C, value: u64) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut buffer = [b'0'; 18];
    buffer[1] = b'x';

    let mut shift = 60u32;
    let mut index = 2usize;
    while index < buffer.len() {
        buffer[index] = HEX[((value >> shift) & 0xF) as usize];
        shift = shift.saturating_sub(4);
        index += 1;
    }

    let text = core::str::from_utf8(&buffer).unwrap_or("0x0000000000000000");
    console.write_str(text);
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_virt_trace"
))]
fn trace_kernel_stage0_marker(message: &[u8]) {
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

#[cfg(not(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_virt_trace"
)))]
fn trace_kernel_stage0_marker(_message: &[u8]) {}

#[cfg(test)]
mod tests {
    use super::{
        BootstrapCapsuleSummary, BootstrapTaskBoundaryPlan, BootstrapTaskLaunchPlan,
        BootstrapTaskSyscallBoundary, BootstrapTaskTransferMode, NovaBootInfoV1, NovaBootInfoV2,
        NovaImageDigestV1, NovaVerificationInfoV1, bootstrap_syscall_dispatch_state,
        bootstrap_task_boundary_plan, bootstrap_task_target_boundary_plan,
        bootstrap_task_transfer_mode, dispatch_bootstrap_kernel_call, prepare_bringup,
        resolve_boot_info, resolve_boot_info_v2, resolve_kernel_image_digest, resolve_memory_map,
        resolve_optional_boot_info_v2, resolve_verification_info,
        run_lower_el_bootstrap_svc_dry_run,
    };
    use crate::bootinfo::{BootSource, FramebufferFormat, NovaBootstrapFrameArenaDescriptorV1};
    use crate::console::ConsoleSink;
    use crate::syscall::{
        BootstrapTaskState, CurrentTaskState, SyscallDispatchState, install_bootstrap_syscall_state,
    };
    use alloc::string::String;
    use nova_rt::{
        InitCapsuleImage, NovaBootstrapTaskContextV1, NovaBootstrapTaskContextV2,
        NovaInitCapsuleCapabilityV1, NovaInitCapsuleHeaderV1, NovaPayloadEntryAbi,
        NovaPayloadHeaderV1, NovaPayloadKind, NovaSyscallNumberV1, NovaSyscallRequestV1,
        NovaSyscallStatusV1, encode_init_capsule_service_name, sha256_digest_bytes,
    };

    #[test]
    fn resolve_boot_info_rejects_invalid_marker() {
        let info = NovaBootInfoV1::empty();
        assert!(resolve_boot_info(&info as *const NovaBootInfoV1).is_none());
    }

    #[test]
    fn resolve_boot_info_accepts_valid_marker() {
        let info = NovaBootInfoV1::new();
        let resolved = resolve_boot_info(&info as *const NovaBootInfoV1).expect("boot info");
        assert!(resolved.is_valid());
    }

    #[test]
    fn resolve_boot_info_v2_rejects_invalid_marker() {
        let info = NovaBootInfoV2::empty();
        assert!(resolve_boot_info_v2(&info as *const NovaBootInfoV2).is_none());
    }

    #[test]
    fn resolve_boot_info_v2_accepts_valid_marker() {
        let info = NovaBootInfoV2::new();
        let resolved = resolve_boot_info_v2(&info as *const NovaBootInfoV2).expect("boot info v2");
        assert!(resolved.is_valid());
    }

    #[test]
    fn resolve_optional_boot_info_v2_accepts_null_sidecar() {
        assert_eq!(resolve_optional_boot_info_v2(core::ptr::null()), Some(None));
    }

    #[test]
    fn prepare_bringup_rejects_invalid_boot_info() {
        let info = NovaBootInfoV1::empty();
        assert!(prepare_bringup(&info, None).is_none());
    }

    #[test]
    fn prepare_bringup_tracks_boot_summary_and_memory_windows() {
        let descriptors = [0xAAu8; 192];
        let digest = NovaImageDigestV1::sha256([0x11; 32]);
        let init_capsule = build_init_capsule();
        let mut verification = NovaVerificationInfoV1::new();
        verification.stage1_image_size = 128;
        verification.kernel_image_size = 4096;
        verification.set_flag(NovaVerificationInfoV1::FLAG_STAGE1_PAYLOAD_PRESENT);
        verification.set_flag(NovaVerificationInfoV1::FLAG_STAGE1_PAYLOAD_VERIFIED);
        verification.set_flag(NovaVerificationInfoV1::FLAG_KERNEL_PAYLOAD_PRESENT);
        verification.set_flag(NovaVerificationInfoV1::FLAG_KERNEL_PAYLOAD_VERIFIED);
        verification.set_flag(NovaVerificationInfoV1::FLAG_KERNEL_DIGEST_PRESENT);
        verification.set_flag(NovaVerificationInfoV1::FLAG_KERNEL_DIGEST_VERIFIED);
        let (info, info_v2) = build_boot_info_pair(
            &descriptors,
            &digest,
            &verification,
            init_capsule.as_slice(),
        );

        let bringup = prepare_bringup(&info, Some(&info_v2)).expect("bringup");

        assert!(bringup.boot_summary.framebuffer_present);
        let v2 = bringup.boot_info_v2.expect("boot info v2");
        let init_capsule_summary = bringup.init_capsule.expect("init capsule");
        assert_eq!(v2.cpu_arch, 1);
        assert_eq!(v2.platform_class, 1);
        assert_eq!(v2.memory_topology_class, 1);
        assert_eq!(v2.boot_source, BootSource::Usb);
        assert!(v2.framebuffer_present);
        assert_eq!(v2.storage_seed_count, 2);
        assert_eq!(v2.network_seed_count, 1);
        assert_eq!(v2.accel_seed_count, 1);
        assert_eq!(bringup.memory_map_bytes, descriptors.len());
        assert!(bringup.kernel_image_digest_present);
        assert!(bringup.verification_info_present);
        assert!(bringup.stage1_payload_verified);
        assert!(bringup.kernel_payload_verified);
        assert_eq!(bringup.boot_summary.memory_map_entries, 4);
        assert_eq!(bringup.page_tables.kernel_base, descriptors.as_ptr() as u64);
        assert_eq!(bringup.page_tables.kernel_size, 4 * 48);
        assert_eq!(bringup.page_tables.user_base, 0x2000);
        assert_eq!(bringup.page_tables.user_size, 3);
        assert_eq!(bringup.page_tables.user_stack_size, 0);
        assert_eq!(bringup.allocator.usable_base, init_capsule.as_ptr() as u64);
        assert_eq!(bringup.allocator.usable_limit, init_capsule_len() as u64);
        assert_eq!(bringup.allocator.reserved_bytes, 0x5000);
        assert_eq!(bringup.allocator.bootstrap_el0_arena_base, 0x9000_0000);
        assert_eq!(bringup.allocator.bootstrap_el0_arena_size, 0x20_000);
        assert_eq!(init_capsule_summary.service_name(), "initd");
        assert_eq!(
            init_capsule_summary.requested_capabilities,
            NovaInitCapsuleCapabilityV1::BootLog as u64
        );
        assert!(!init_capsule_summary.has_payload());
    }

    #[test]
    fn prepare_bringup_tracks_embedded_bootstrap_task_payload() {
        let descriptors = [0xCCu8; 192];
        let digest = NovaImageDigestV1::sha256([0x33; 32]);
        let init_capsule = build_init_capsule_with_payload();
        let mut verification = NovaVerificationInfoV1::new();
        verification.set_flag(NovaVerificationInfoV1::FLAG_STAGE1_PAYLOAD_PRESENT);
        verification.set_flag(NovaVerificationInfoV1::FLAG_STAGE1_PAYLOAD_VERIFIED);
        verification.set_flag(NovaVerificationInfoV1::FLAG_KERNEL_PAYLOAD_PRESENT);
        verification.set_flag(NovaVerificationInfoV1::FLAG_KERNEL_PAYLOAD_VERIFIED);
        verification.set_flag(NovaVerificationInfoV1::FLAG_KERNEL_DIGEST_PRESENT);
        verification.set_flag(NovaVerificationInfoV1::FLAG_KERNEL_DIGEST_VERIFIED);
        verification.kernel_image_size = 4096;
        let (info, info_v2) = build_boot_info_pair(
            &descriptors,
            &digest,
            &verification,
            init_capsule.as_slice(),
        );

        let bringup = prepare_bringup(&info, Some(&info_v2)).expect("bringup");
        let init_capsule_summary = bringup.init_capsule.expect("init capsule");

        assert!(init_capsule_summary.has_payload());
        assert!(init_capsule_summary.payload_descriptor_from_boot_info_v2);
        assert_eq!(
            init_capsule_summary.payload_image_size,
            init_capsule.len() as u64 - 64
        );
        assert_eq!(init_capsule_summary.payload_load_size, 4);
        assert_eq!(
            init_capsule_summary.payload_entry_point,
            init_capsule_summary.payload_load_base
        );
        assert_eq!(
            init_capsule_summary.launch_plan(),
            Some(BootstrapTaskLaunchPlan {
                service_name: encode_init_capsule_service_name("initd").expect("service name"),
                image_base: init_capsule_summary.payload_image_base,
                image_size: init_capsule_summary.payload_image_size,
                load_base: init_capsule_summary.payload_load_base,
                load_size: 4,
                entry_point: init_capsule_summary.payload_entry_point,
            })
        );
    }

    #[test]
    fn prepare_bringup_rejects_mismatched_bootstrap_payload_sidecar() {
        let descriptors = [0x66u8; 192];
        let digest = NovaImageDigestV1::sha256([0x44; 32]);
        let init_capsule = build_init_capsule_with_payload();
        let mut verification = NovaVerificationInfoV1::new();
        verification.set_flag(NovaVerificationInfoV1::FLAG_STAGE1_PAYLOAD_PRESENT);
        verification.set_flag(NovaVerificationInfoV1::FLAG_KERNEL_PAYLOAD_PRESENT);
        verification.set_flag(NovaVerificationInfoV1::FLAG_KERNEL_PAYLOAD_VERIFIED);
        verification.kernel_image_size = 4096;
        let (info, mut info_v2) = build_boot_info_pair(
            &descriptors,
            &digest,
            &verification,
            init_capsule.as_slice(),
        );
        info_v2.bootstrap_payload.image_len -= 1;

        assert!(prepare_bringup(&info, Some(&info_v2)).is_none());
    }

    #[test]
    fn prepare_bringup_rejects_mismatched_boot_info_v2_sidecar() {
        let descriptors = [0x55u8; 192];
        let digest = NovaImageDigestV1::sha256([0x22; 32]);
        let init_capsule = build_init_capsule();
        let mut verification = NovaVerificationInfoV1::new();
        verification.set_flag(NovaVerificationInfoV1::FLAG_STAGE1_PAYLOAD_PRESENT);
        let (info, mut info_v2) = build_boot_info_pair(
            &descriptors,
            &digest,
            &verification,
            init_capsule.as_slice(),
        );
        info_v2.config_table_count += 1;

        assert!(prepare_bringup(&info, Some(&info_v2)).is_none());
    }

    #[test]
    fn bootstrap_syscall_state_marks_endpoint_lane_ready_from_capsule_bootstrap_state() {
        let state = bootstrap_syscall_dispatch_state(Some(BootstrapCapsuleSummary {
            service_name: encode_init_capsule_service_name("initd").expect("service name"),
            requested_capabilities: (NovaInitCapsuleCapabilityV1::BootLog as u64)
                | (NovaInitCapsuleCapabilityV1::EndpointBootstrap as u64),
            endpoint_slots: 2,
            shared_memory_regions: 1,
            payload_body_present: false,
            payload_image_base: 0,
            payload_image_size: 0,
            payload_load_base: 0,
            payload_load_size: 0,
            payload_entry_point: 0,
            payload_descriptor_from_boot_info_v2: false,
        }));

        assert_eq!(
            state,
            SyscallDispatchState::bootstrap(
                CurrentTaskState::new(
                    encode_init_capsule_service_name("initd").expect("service name"),
                    BootstrapTaskState::new(
                        (NovaInitCapsuleCapabilityV1::BootLog as u64)
                            | (NovaInitCapsuleCapabilityV1::EndpointBootstrap as u64),
                        2,
                        1,
                    ),
                ),
                true,
                false,
            )
        );
    }

    #[test]
    fn bootstrap_syscall_state_marks_shared_memory_lane_ready_from_capsule_bootstrap_state() {
        let state = bootstrap_syscall_dispatch_state(Some(BootstrapCapsuleSummary {
            service_name: encode_init_capsule_service_name("initd").expect("service name"),
            requested_capabilities: (NovaInitCapsuleCapabilityV1::BootLog as u64)
                | (NovaInitCapsuleCapabilityV1::SharedMemoryBootstrap as u64),
            endpoint_slots: 0,
            shared_memory_regions: 2,
            payload_body_present: false,
            payload_image_base: 0,
            payload_image_size: 0,
            payload_load_base: 0,
            payload_load_size: 0,
            payload_entry_point: 0,
            payload_descriptor_from_boot_info_v2: false,
        }));

        assert_eq!(
            state,
            SyscallDispatchState::bootstrap(
                CurrentTaskState::new(
                    encode_init_capsule_service_name("initd").expect("service name"),
                    BootstrapTaskState::new(
                        (NovaInitCapsuleCapabilityV1::BootLog as u64)
                            | (NovaInitCapsuleCapabilityV1::SharedMemoryBootstrap as u64),
                        0,
                        2,
                    ),
                ),
                false,
                true,
            )
        );
    }

    #[test]
    fn bootstrap_task_transfer_mode_only_drops_from_el2() {
        assert_eq!(
            bootstrap_task_transfer_mode(1),
            BootstrapTaskTransferMode::SameEl
        );
        assert_eq!(
            bootstrap_task_transfer_mode(2),
            BootstrapTaskTransferMode::DropToEl1
        );
        assert_eq!(
            bootstrap_task_transfer_mode(3),
            BootstrapTaskTransferMode::SameEl
        );
    }

    #[test]
    fn bootstrap_task_boundary_plan_marks_same_el_as_unisolated_current_el_svc() {
        assert_eq!(
            bootstrap_task_boundary_plan(1),
            BootstrapTaskBoundaryPlan {
                current_el: 1,
                target_el: 1,
                transfer_mode: BootstrapTaskTransferMode::SameEl,
                task_isolated: false,
                syscall_boundary: BootstrapTaskSyscallBoundary::CurrentElSvc,
            }
        );
        assert_eq!(BootstrapTaskTransferMode::SameEl.label(), "same-el");
        assert_eq!(
            BootstrapTaskSyscallBoundary::CurrentElSvc.label(),
            "current-el-svc"
        );
        assert_eq!(BootstrapTaskSyscallBoundary::El0Svc.label(), "el0-svc");
    }

    #[test]
    fn bootstrap_task_boundary_plan_tracks_el2_drop_without_claiming_el0() {
        assert_eq!(
            bootstrap_task_boundary_plan(2),
            BootstrapTaskBoundaryPlan {
                current_el: 2,
                target_el: 1,
                transfer_mode: BootstrapTaskTransferMode::DropToEl1,
                task_isolated: false,
                syscall_boundary: BootstrapTaskSyscallBoundary::CurrentElSvc,
            }
        );
    }

    #[test]
    fn bootstrap_task_target_boundary_plan_tracks_isolated_el0_goal() {
        assert_eq!(
            bootstrap_task_target_boundary_plan(1),
            BootstrapTaskBoundaryPlan {
                current_el: 1,
                target_el: 0,
                transfer_mode: BootstrapTaskTransferMode::DropToEl0,
                task_isolated: true,
                syscall_boundary: BootstrapTaskSyscallBoundary::El0Svc,
            }
        );
        assert_eq!(BootstrapTaskTransferMode::DropToEl0.label(), "drop-to-el0");
    }

    #[test]
    fn lower_el_bootstrap_svc_dry_run_proves_elr_advance_path() {
        let service_name = encode_init_capsule_service_name("initd").expect("service name");
        let state = SyscallDispatchState::bootstrap(
            CurrentTaskState::new(
                service_name,
                BootstrapTaskState::new(NovaInitCapsuleCapabilityV1::BootLog as u64, 0, 0),
            ),
            false,
            false,
        );
        let mut console = RecordingConsole::new();

        run_lower_el_bootstrap_svc_dry_run(&mut console, &state);

        assert!(
            console
                .as_str()
                .contains("bootstrap lower-el svc from initd")
        );
        assert!(
            console
                .as_str()
                .contains("bootstrap lower-el svc dry-run passed")
        );
    }

    #[test]
    fn bootstrap_kernel_call_round_trips_trace_request() {
        let service_name = encode_init_capsule_service_name("initd").expect("service name");
        install_bootstrap_syscall_state(SyscallDispatchState::bootstrap(
            CurrentTaskState::new(
                service_name,
                BootstrapTaskState::new(NovaInitCapsuleCapabilityV1::BootLog as u64, 1, 0),
            ),
            true,
            false,
        ));
        let context = NovaBootstrapTaskContextV2::new(
            service_name,
            NovaInitCapsuleCapabilityV1::BootLog as u64,
            1,
            0,
            1,
        );
        let request = NovaSyscallRequestV1::new(
            NovaSyscallNumberV1::Trace,
            0,
            [0xCAFE_BABE, 0x5151_0001, 0, 0, 0, 0],
        );
        let mut console = RecordingConsole::new();

        let result = dispatch_bootstrap_kernel_call(
            &context as *const NovaBootstrapTaskContextV2 as *const NovaBootstrapTaskContextV1,
            request,
            &mut console,
        );

        assert_eq!(result.status, NovaSyscallStatusV1::Ok as u32);
        assert_eq!(result.value0, 0xCAFE_BABE);
        assert_eq!(result.value1, 0x5151_0001);
        assert!(
            console
                .as_str()
                .contains("bootstrap kernel call from initd")
        );
        assert!(
            console
                .as_str()
                .contains("syscall trace request from initd")
        );
    }

    #[test]
    fn bootstrap_kernel_call_rejects_mismatched_task_identity() {
        let service_name = encode_init_capsule_service_name("initd").expect("service name");
        install_bootstrap_syscall_state(SyscallDispatchState::bootstrap(
            CurrentTaskState::new(
                service_name,
                BootstrapTaskState::new(NovaInitCapsuleCapabilityV1::BootLog as u64, 1, 0),
            ),
            true,
            false,
        ));
        let foreign_context = NovaBootstrapTaskContextV2::new(
            encode_init_capsule_service_name("shell").expect("service name"),
            NovaInitCapsuleCapabilityV1::BootLog as u64,
            1,
            0,
            1,
        );
        let mut console = RecordingConsole::new();

        let result = dispatch_bootstrap_kernel_call(
            &foreign_context as *const NovaBootstrapTaskContextV2
                as *const NovaBootstrapTaskContextV1,
            NovaSyscallRequestV1::new(NovaSyscallNumberV1::Trace, 0, [1, 2, 0, 0, 0, 0]),
            &mut console,
        );

        assert_eq!(result.status, NovaSyscallStatusV1::Denied as u32);
        assert_eq!(console.as_str(), "");
    }

    #[test]
    fn resolve_memory_map_rejects_missing_window() {
        let info = NovaBootInfoV1::new();
        assert!(resolve_memory_map(&info).is_none());
    }

    #[test]
    fn resolve_kernel_image_digest_rejects_missing_pointer() {
        let info = NovaBootInfoV1::new();
        assert!(resolve_kernel_image_digest(&info).is_none());
    }

    #[test]
    fn resolve_verification_info_rejects_missing_pointer() {
        let info = NovaBootInfoV1::new();
        assert!(resolve_verification_info(&info).is_none());
    }

    fn build_boot_info_pair(
        descriptors: &[u8; 192],
        digest: &NovaImageDigestV1,
        verification: &NovaVerificationInfoV1,
        init_capsule: &[u8],
    ) -> (NovaBootInfoV1, NovaBootInfoV2) {
        let mut info = NovaBootInfoV1::new();
        info.firmware_vendor_ptr = 0x1111;
        info.firmware_revision = 42;
        info.secure_boot_state = NovaBootInfoV1::SECURE_BOOT_ENABLED;
        info.boot_source = BootSource::Usb;
        info.current_el = 2;
        info.memory_map_ptr = descriptors.as_ptr() as u64;
        info.memory_map_entries = 4;
        info.memory_map_desc_size = 48;
        info.config_tables_ptr = 0x2000;
        info.config_table_count = 3;
        info.acpi_rsdp_ptr = 0x2100;
        info.dtb_ptr = 0x2200;
        info.smbios_ptr = 0x2300;
        info.init_capsule_ptr = init_capsule.as_ptr() as u64;
        info.init_capsule_len = init_capsule.len() as u64;
        info.loader_log_ptr = 0x5000;
        info.framebuffer_base = 0x6000;
        info.framebuffer_width = 1920;
        info.framebuffer_height = 1080;
        info.framebuffer_stride = 1920;
        info.framebuffer_format = FramebufferFormat::Rgbx8888;
        info.set_flag(NovaBootInfoV1::FLAG_HAS_FRAMEBUFFER);
        info.set_flag(NovaBootInfoV1::FLAG_HAS_KERNEL_IMAGE_DIGEST);
        info.set_flag(NovaBootInfoV1::FLAG_HAS_VERIFICATION_INFO);
        info.kernel_image_hash_ptr = digest as *const NovaImageDigestV1 as u64;
        info.verification_info_ptr = verification as *const NovaVerificationInfoV1 as u64;

        let mut info_v2 = NovaBootInfoV2::new();
        info_v2.cpu_arch = unsafe { core::mem::transmute::<u16, _>(1) };
        info_v2.platform_class = unsafe { core::mem::transmute::<u16, _>(1) };
        info_v2.memory_topology_class = unsafe { core::mem::transmute::<u16, _>(1) };
        info_v2.firmware_vendor_ptr = info.firmware_vendor_ptr;
        info_v2.firmware_revision = info.firmware_revision;
        info_v2.secure_boot_state = info.secure_boot_state;
        info_v2.boot_source = info.boot_source;
        info_v2.current_el = info.current_el;
        info_v2.memory_map_ptr = info.memory_map_ptr;
        info_v2.memory_map_entries = info.memory_map_entries;
        info_v2.memory_map_desc_size = info.memory_map_desc_size;
        info_v2.config_tables_ptr = info.config_tables_ptr;
        info_v2.config_table_count = info.config_table_count;
        info_v2.acpi_rsdp_ptr = info.acpi_rsdp_ptr;
        info_v2.dtb_ptr = info.dtb_ptr;
        info_v2.smbios_ptr = info.smbios_ptr;
        info_v2.framebuffer.base = info.framebuffer_base;
        info_v2.framebuffer.width = info.framebuffer_width;
        info_v2.framebuffer.height = info.framebuffer_height;
        info_v2.framebuffer.stride = info.framebuffer_stride;
        info_v2.framebuffer.format = info.framebuffer_format;
        info_v2.storage_seed_count = 2;
        info_v2.network_seed_count = 1;
        info_v2.accel_seed_count = 1;
        info_v2.init_capsule_ptr = info.init_capsule_ptr;
        info_v2.init_capsule_len = info.init_capsule_len;
        info_v2.loader_log_ptr = info.loader_log_ptr;
        info_v2.kernel_image_hash_ptr = info.kernel_image_hash_ptr;
        info_v2.bootstrap_frame_arena = NovaBootstrapFrameArenaDescriptorV1 {
            base: 0x9000_0000,
            len: 0x20_000,
            page_size: NovaBootstrapFrameArenaDescriptorV1::PAGE_SIZE_4K,
            flags: 0,
        };
        if let Some(payload) = InitCapsuleImage::parse(init_capsule)
            .and_then(|capsule| capsule.bootstrap_service_payload())
        {
            let image = payload.image_bytes();
            let image_base = image.as_ptr() as u64;
            info_v2.bootstrap_payload.image_ptr = image_base;
            info_v2.bootstrap_payload.image_len = image.len() as u64;
            info_v2.bootstrap_payload.load_base = payload.load_base(image_base);
            info_v2.bootstrap_payload.load_size = payload.load_size();
            info_v2.bootstrap_payload.entry_point = payload.entry_addr(image_base);
        }

        (info, info_v2)
    }

    fn build_init_capsule() -> [u8; core::mem::size_of::<NovaInitCapsuleHeaderV1>()] {
        let header = NovaInitCapsuleHeaderV1::new(
            encode_init_capsule_service_name("initd").expect("service name"),
            NovaInitCapsuleCapabilityV1::BootLog as u64,
            0,
            0,
        );
        let mut image = [0u8; core::mem::size_of::<NovaInitCapsuleHeaderV1>()];
        image.copy_from_slice(unsafe {
            core::slice::from_raw_parts(
                &header as *const NovaInitCapsuleHeaderV1 as *const u8,
                core::mem::size_of::<NovaInitCapsuleHeaderV1>(),
            )
        });
        image
    }

    fn build_init_capsule_with_payload() -> alloc::vec::Vec<u8> {
        let payload_body = [0x11u8, 0x22, 0x33, 0x44];
        let payload_header = NovaPayloadHeaderV1::new_flat_binary(
            NovaPayloadKind::Service,
            NovaPayloadEntryAbi::BootstrapTaskV1,
            (core::mem::size_of::<NovaPayloadHeaderV1>() + payload_body.len()) as u32,
            sha256_digest_bytes(&payload_body),
        );
        let mut payload =
            alloc::vec![0u8; core::mem::size_of::<NovaPayloadHeaderV1>() + payload_body.len()];
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

        let mut image = alloc::vec![0u8; header.total_size as usize];
        image[..core::mem::size_of::<NovaInitCapsuleHeaderV1>()].copy_from_slice(unsafe {
            core::slice::from_raw_parts(
                &header as *const NovaInitCapsuleHeaderV1 as *const u8,
                core::mem::size_of::<NovaInitCapsuleHeaderV1>(),
            )
        });
        image[core::mem::size_of::<NovaInitCapsuleHeaderV1>()..].copy_from_slice(&payload);
        image
    }

    const fn init_capsule_len() -> usize {
        core::mem::size_of::<NovaInitCapsuleHeaderV1>()
    }

    struct RecordingConsole {
        output: String,
    }

    impl RecordingConsole {
        fn new() -> Self {
            Self {
                output: String::new(),
            }
        }

        fn as_str(&self) -> &str {
            self.output.as_str()
        }
    }

    impl ConsoleSink for RecordingConsole {
        fn write_str(&mut self, text: &str) {
            self.output.push_str(text);
        }
    }
}
