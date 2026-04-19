#![cfg_attr(target_os = "uefi", no_std)]
#![cfg_attr(target_os = "uefi", no_main)]

mod bootinfo;

use bootinfo::LoaderHandoff;
#[cfg(not(target_os = "uefi"))]
use bootinfo::LoaderPlan;

#[cfg(target_os = "uefi")]
use uefi::prelude::*;
#[cfg(target_os = "uefi")]
use uefi::{boot, println};

#[cfg(target_os = "uefi")]
#[entry]
fn efi_main() -> Status {
    let _ = uefi::helpers::init();
    trace_pre_exit_stage0();

    let handoff = LoaderHandoff::from_uefi();
    if let Err(error) = handoff.validate() {
        println!("NovaOS stage0 error={error:?}");
        boot::stall(2_000_000);
        return Status::LOAD_ERROR;
    }

    let handoff_report = handoff.structured_handoff_report();
    let handoff_report_path = handoff.persist_handoff_report();
    println!("{}", handoff.summary());
    if let Some(path) = handoff_report_path.as_deref() {
        println!("loader_handoff_report_path={path}");
    } else {
        println!("loader_handoff_report_path=write_failed");
    }
    println!("loader_handoff_report_begin");
    for line in handoff_report.lines() {
        println!("{}", line);
    }
    println!("loader_handoff_report_end");
    boot::stall(500_000);

    let post_exit = handoff.exit_boot_services_and_prepare_stage1();
    post_exit.run()
}

#[cfg(not(target_os = "uefi"))]
fn main() {
    let handoff = LoaderHandoff::prepare(LoaderPlan::unknown());
    println!("{}", handoff.summary());
}

#[cfg(all(
    target_os = "uefi",
    target_arch = "aarch64",
    feature = "qemu_virt_trace"
))]
#[allow(dead_code)]
fn trace_pre_exit_stage0() {
    const PL011_BASE: usize = 0x0900_0000;
    const PL011_DR: *mut u32 = PL011_BASE as *mut u32;
    const PL011_FR: *const u32 = (PL011_BASE + 0x18) as *const u32;
    const PL011_FR_TXFF: u32 = 1 << 5;

    for &byte in b"NovaOS stage0 pre-exit\n" {
        while unsafe { core::ptr::read_volatile(PL011_FR) } & PL011_FR_TXFF != 0 {}
        unsafe {
            core::ptr::write_volatile(PL011_DR, byte as u32);
        }
    }
}

#[cfg(all(
    target_os = "uefi",
    not(all(target_arch = "aarch64", feature = "qemu_virt_trace"))
))]
#[allow(dead_code)]
fn trace_pre_exit_stage0() {}
