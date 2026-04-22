#![no_std]
#![cfg_attr(
    feature = "bootstrap_kernel_svc_probe",
    allow(dead_code, unreachable_code)
)]

#[cfg(test)]
extern crate alloc;

pub mod arch;
pub mod boot_contract;
pub mod bootinfo;
pub mod bootstrap;
pub mod bringup;
pub mod console;
pub mod diag;
pub mod el;
pub mod exception_runtime;
pub mod mm;
pub mod panic;
pub mod syscall;
pub mod trace;

pub use boot_contract::{
    BootstrapCapsuleSummary, BootstrapTaskLaunchPlan, KernelBringupState, KernelBringupV2State,
    prepare_bringup, resolve_boot_info, resolve_boot_info_v2, resolve_kernel_image_digest,
    resolve_memory_map, resolve_optional_boot_info_v2, resolve_verification_info,
};
use bootinfo::{NovaBootInfoV1, NovaBootInfoV2};
use bringup::enter_kernel_runtime;
pub use bringup::{KernelContext, kernel_main};
pub(crate) use diag::trace_kernel_stage0_marker;

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

#[cfg(test)]
mod tests;
