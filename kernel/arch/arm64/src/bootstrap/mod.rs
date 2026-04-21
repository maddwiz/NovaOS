#[cfg(all(target_os = "none", target_arch = "aarch64"))]
use crate::arch::arm64::allocator::BootstrapEl0BackingFramePlan;
use crate::arch::arm64::allocator::FrameAllocatorPlan;
use crate::arch::arm64::mmu::PageTablePlan;
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
use crate::arch::arm64::mmu::{
    BootstrapEl0BackingFramePopulation, BootstrapEl0BackingFramePopulationReadiness,
    BootstrapEl0MappingRequest, BootstrapEl0PageTableConstruction, BootstrapEl0PageTablePlan,
    construct_bootstrap_el0_page_tables, populate_bootstrap_el0_backing_frames,
};
use crate::boot_contract::{BootstrapCapsuleSummary, BootstrapTaskLaunchPlan};
use crate::console::ConsoleSink;
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
use crate::console::TraceConsole;
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
use crate::diag::write_hex_u64;
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_pretransfer_svc_probe"
))]
use crate::diag::{
    log_runtime_exception_probe_state, read_runtime_exception_probe_state,
    trace_kernel_stage0_marker,
};
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
use crate::el::{
    BOOTSTRAP_TASK_STACK_SIZE, BootstrapTaskBoundaryPlan, BootstrapTaskEntry,
    bootstrap_task_boundary_plan, bootstrap_task_target_boundary_plan,
    enter_bootstrap_task_with_stack, read_runtime_current_el,
};
use crate::syscall::{bootstrap_syscall_state, dispatch_syscall};
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
use core::mem::size_of;
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
use nova_rt::NovaBootstrapTaskContextV2;
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_pretransfer_svc_probe"
))]
use nova_rt::NovaSyscallStatusV1;
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_pretransfer_svc_probe"
))]
use nova_rt::syscall::trace;
use nova_rt::{
    NovaBootstrapTaskContextV1, NovaSyscallRequestV1, NovaSyscallResultV1,
    resolve_bootstrap_task_context,
};

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
static mut BOOTSTRAP_TASK_CONTEXT: NovaBootstrapTaskContextV2 = NovaBootstrapTaskContextV2::empty();

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
pub(crate) fn enter_bootstrap_task<C: ConsoleSink>(
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
pub(crate) fn enter_bootstrap_task<C: ConsoleSink>(
    console: &mut C,
    _launch_plan: BootstrapTaskLaunchPlan,
    _init_capsule: Option<BootstrapCapsuleSummary>,
    _page_tables: PageTablePlan,
    _allocator: FrameAllocatorPlan,
) -> ! {
    crate::panic::log_and_halt(
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
        console.write_str("[info] bootstrap el0 backing frames populated ");
        let population = if page_table_plan.ready() {
            let population = unsafe { populate_live_bootstrap_el0_backing_frames(page_table_plan) };
            console.write_line(population.readiness.label());
            population
        } else {
            console.write_line("page-tables-not-ready");
            BootstrapEl0BackingFramePopulation {
                readiness: BootstrapEl0BackingFramePopulationReadiness::PageTablePlanNotReady,
                source_readiness: page_table_plan.readiness,
                payload_bytes: 0,
                context_bytes: 0,
                zeroed_bytes: 0,
            }
        };
        console.write_str("[info] bootstrap el0 page tables constructed ");
        if page_table_plan.ready() && population.ready() {
            let construction = unsafe { construct_live_bootstrap_el0_page_tables(page_table_plan) };
            console.write_line(construction.readiness.label());
            console.write_str("[info] bootstrap el0 mmu registers prepared ");
            let registers = page_table_plan.mmu_register_plan(construction);
            console.write_line(registers.readiness.label());
        } else {
            console.write_line("backing-frames-not-populated");
            console.write_line(
                "[info] bootstrap el0 mmu registers prepared page-tables-not-constructed",
            );
        }
    } else {
        console.write_line("backing-frames-not-ready");
        console.write_line("[info] bootstrap el0 backing frames populated page-tables-not-ready");
        console.write_line("[info] bootstrap el0 page tables constructed page-tables-not-ready");
        console.write_line("[info] bootstrap el0 mmu registers prepared page-tables-not-ready");
    }
}

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
unsafe fn populate_live_bootstrap_el0_backing_frames(
    plan: BootstrapEl0PageTablePlan,
) -> BootstrapEl0BackingFramePopulation {
    let payload_source = unsafe {
        core::slice::from_raw_parts(
            plan.payload_copy_source_base as *const u8,
            plan.payload_copy_source_size as usize,
        )
    };
    let context_source = unsafe {
        core::slice::from_raw_parts(
            plan.context_copy_source_base as *const u8,
            plan.context_copy_source_size as usize,
        )
    };
    let image_frame = unsafe {
        core::slice::from_raw_parts_mut(
            plan.user_image_mapping.phys_base as *mut u8,
            plan.user_image_mapping.size as usize,
        )
    };
    let context_frame = unsafe {
        core::slice::from_raw_parts_mut(
            plan.user_context_mapping.phys_base as *mut u8,
            plan.user_context_mapping.size as usize,
        )
    };
    let stack_frame = unsafe {
        core::slice::from_raw_parts_mut(
            plan.user_stack_mapping.phys_base as *mut u8,
            plan.user_stack_mapping.size as usize,
        )
    };

    let population = populate_bootstrap_el0_backing_frames(
        plan,
        payload_source,
        context_source,
        image_frame,
        context_frame,
        stack_frame,
    );
    if population.ready() {
        sync_instruction_cache(
            plan.user_image_mapping.phys_base as *const u8,
            plan.user_image_mapping.size as usize,
        );
        clean_data_cache(
            plan.user_context_mapping.phys_base as *const u8,
            plan.user_context_mapping.size as usize,
        );
        clean_data_cache(
            plan.user_stack_mapping.phys_base as *const u8,
            plan.user_stack_mapping.size as usize,
        );
    }
    population
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
pub(crate) fn dispatch_bootstrap_kernel_call<C: ConsoleSink>(
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
    feature = "bootstrap_pretransfer_svc_probe"
))]
unsafe extern "C" fn bootstrap_pretransfer_svc_probe_entry(
    context: *const NovaBootstrapTaskContextV1,
) -> ! {
    const TRACE_VALUE0: u64 = 0x5052_4553_5643_3031;
    const TRACE_VALUE1: u64 = 0x4E4F_5641_5052_4554;

    if resolve_bootstrap_task_context(context).is_none() {
        trace_kernel_stage0_marker(b"NovaOS bootstrap pretransfer svc invalid context\n");
        crate::panic::halt();
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

    crate::panic::halt();
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
