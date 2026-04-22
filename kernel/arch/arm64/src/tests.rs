use super::{
    BootstrapCapsuleSummary, BootstrapTaskLaunchPlan, NovaBootInfoV1, NovaBootInfoV2,
    prepare_bringup, resolve_boot_info, resolve_boot_info_v2, resolve_kernel_image_digest,
    resolve_memory_map, resolve_optional_boot_info_v2, resolve_verification_info,
};
use crate::bootinfo::{
    BootSource, FramebufferFormat, NovaBootstrapFrameArenaDescriptorV1, NovaImageDigestV1,
    NovaVerificationInfoV1,
};
use crate::bootstrap::dispatch_bootstrap_kernel_call;
use crate::console::ConsoleSink;
use crate::diag::run_lower_el_bootstrap_svc_dry_run;
use crate::el::{
    BootstrapTaskBoundaryPlan, BootstrapTaskSyscallBoundary, BootstrapTaskTransferMode,
    bootstrap_task_boundary_plan, bootstrap_task_target_boundary_plan,
    bootstrap_task_transfer_mode,
};
use crate::exception_runtime::bootstrap_syscall_dispatch_state;
use crate::syscall::{
    BootstrapTaskState, CurrentTaskState, SyscallDispatchState, install_bootstrap_syscall_state,
};
use alloc::string::String;
use nova_rt::{
    InitCapsuleImage, NovaBootstrapTaskContextV1, NovaBootstrapTaskContextV2,
    NovaInitCapsuleCapabilityV1, NovaInitCapsuleHeaderV1, NovaPayloadEntryAbi, NovaPayloadHeaderV1,
    NovaPayloadKind, NovaSyscallNumberV1, NovaSyscallRequestV1, NovaSyscallStatusV1,
    encode_init_capsule_service_name, sha256_digest_bytes,
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
        &foreign_context as *const NovaBootstrapTaskContextV2 as *const NovaBootstrapTaskContextV1,
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
    header.total_size = (core::mem::size_of::<NovaInitCapsuleHeaderV1>() + payload.len()) as u32;

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
