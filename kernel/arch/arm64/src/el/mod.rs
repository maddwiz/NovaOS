#[cfg(all(target_os = "none", target_arch = "aarch64"))]
use nova_rt::NovaBootstrapTaskContextV1;

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
pub(crate) type BootstrapTaskEntry = unsafe extern "C" fn(*const NovaBootstrapTaskContextV1) -> !;

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
pub(crate) const BOOTSTRAP_TASK_STACK_SIZE: usize = 64 * 1024;
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

#[cfg(any(test, all(target_os = "none", target_arch = "aarch64")))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BootstrapTaskTransferMode {
    SameEl,
    DropToEl1,
    DropToEl0,
}

#[cfg(any(test, all(target_os = "none", target_arch = "aarch64")))]
impl BootstrapTaskTransferMode {
    pub(crate) const fn label(self) -> &'static str {
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
pub(crate) enum BootstrapTaskSyscallBoundary {
    CurrentElSvc,
    El0Svc,
}

#[cfg(any(test, all(target_os = "none", target_arch = "aarch64")))]
impl BootstrapTaskSyscallBoundary {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::CurrentElSvc => "current-el-svc",
            Self::El0Svc => "el0-svc",
        }
    }
}

#[cfg(any(test, all(target_os = "none", target_arch = "aarch64")))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct BootstrapTaskBoundaryPlan {
    pub(crate) current_el: u8,
    pub(crate) target_el: u8,
    pub(crate) transfer_mode: BootstrapTaskTransferMode,
    pub(crate) task_isolated: bool,
    pub(crate) syscall_boundary: BootstrapTaskSyscallBoundary,
}

#[cfg(any(test, all(target_os = "none", target_arch = "aarch64")))]
pub(crate) const fn bootstrap_task_transfer_mode(current_el: u8) -> BootstrapTaskTransferMode {
    if current_el == 2 {
        BootstrapTaskTransferMode::DropToEl1
    } else {
        BootstrapTaskTransferMode::SameEl
    }
}

#[cfg(any(test, all(target_os = "none", target_arch = "aarch64")))]
pub(crate) const fn bootstrap_task_boundary_plan(current_el: u8) -> BootstrapTaskBoundaryPlan {
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
pub(crate) const fn bootstrap_task_target_boundary_plan(
    current_el: u8,
) -> BootstrapTaskBoundaryPlan {
    BootstrapTaskBoundaryPlan {
        current_el,
        target_el: 0,
        transfer_mode: BootstrapTaskTransferMode::DropToEl0,
        task_isolated: true,
        syscall_boundary: BootstrapTaskSyscallBoundary::El0Svc,
    }
}

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
pub(crate) unsafe fn enter_bootstrap_task_with_stack(
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
        BootstrapTaskTransferMode::DropToEl0 => crate::panic::halt(),
    }
}

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
pub(crate) fn read_runtime_current_el() -> u8 {
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
pub(crate) fn read_runtime_vbar_el1() -> u64 {
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
