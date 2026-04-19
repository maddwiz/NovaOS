#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

#[cfg(target_os = "none")]
use core::hint::spin_loop;
#[cfg(target_os = "none")]
use core::panic::PanicInfo;
#[cfg(target_os = "none")]
use novaos_kernel::bootinfo::{NovaBootInfoV1, NovaBootInfoV2};

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_virt_trace"
))]
const KERNEL_TRACE: &[u8] = b"NovaOS kernel entered\n";
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_semihosting",
    not(feature = "qemu_virt_trace")
))]
const KERNEL_TRACE: &[u8] = b"NovaOS kernel entered\n\0";

#[cfg(target_os = "none")]
#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {
        spin_loop();
    }
}

#[cfg(target_os = "none")]
#[unsafe(no_mangle)]
pub extern "C" fn _start(
    boot_info: *const NovaBootInfoV1,
    boot_info_v2: *const NovaBootInfoV2,
) -> ! {
    trace_kernel_entry();
    novaos_kernel::kernel_stage0_entry(boot_info, boot_info_v2)
}

#[cfg(not(target_os = "none"))]
fn main() {
    println!("{}", novaos_kernel::kernel_identity());
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_virt_trace"
))]
#[allow(dead_code)]
fn trace_kernel_entry() {
    qemu_uart_write(KERNEL_TRACE);
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_semihosting",
    not(feature = "qemu_virt_trace")
))]
#[allow(dead_code)]
fn trace_kernel_entry() {
    semihost_write0(KERNEL_TRACE);
}

#[cfg(all(
    target_os = "none",
    not(any(
        all(target_arch = "aarch64", feature = "qemu_virt_trace"),
        all(
            target_arch = "aarch64",
            feature = "qemu_semihosting",
            not(feature = "qemu_virt_trace")
        )
    ))
))]
#[allow(dead_code)]
fn trace_kernel_entry() {}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_virt_trace"
))]
#[allow(dead_code)]
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

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_semihosting",
    not(feature = "qemu_virt_trace")
))]
#[allow(dead_code)]
fn semihost_write0(message: &[u8]) {
    let ptr = message.as_ptr();
    unsafe {
        core::arch::asm!(
            "hlt #0xf000",
            in("x0") 0x04usize,
            in("x1") ptr,
            options(nostack, preserves_flags),
        );
    }
}
