#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

#[cfg(target_os = "none")]
use core::hint::spin_loop;
#[cfg(target_os = "none")]
use core::panic::PanicInfo;
#[cfg(target_os = "none")]
use nova_rt::NovaBootstrapTaskContextV1;
#[cfg(all(target_os = "none", not(feature = "bootstrap_el0_probe")))]
use nova_rt::bootstrap_trace;
#[cfg(all(target_os = "none", feature = "bootstrap_trap_probe"))]
use nova_rt::syscall::bootstrap_trap_trace;
#[cfg(all(target_os = "none", feature = "bootstrap_svc_probe"))]
use nova_rt::syscall::trace;
#[cfg(all(target_os = "none", not(feature = "bootstrap_el0_probe")))]
use nova_rt::{NovaSyscallStatusV1, resolve_bootstrap_task_context};

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_virt_trace"
))]
const INITD_TRACE: &[u8] = b"NovaOS initd payload entered\n";
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_virt_trace"
))]
const INITD_CONTEXT_OK_TRACE: &[u8] = b"NovaOS initd bootstrap context ready\n";
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_virt_trace"
))]
const INITD_KERNEL_CALL_OK_TRACE: &[u8] = b"NovaOS initd bootstrap kernel call passed\n";
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_virt_trace"
))]
const INITD_KERNEL_CALL_FAILED_TRACE: &[u8] = b"NovaOS initd bootstrap kernel call failed\n";
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_virt_trace",
    feature = "bootstrap_svc_probe"
))]
const INITD_SVC_BEGIN_TRACE: &[u8] = b"NovaOS initd bootstrap svc begin\n";
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_virt_trace",
    feature = "bootstrap_svc_probe"
))]
const INITD_SVC_OK_TRACE: &[u8] = b"NovaOS initd bootstrap svc passed\n";
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_virt_trace",
    feature = "bootstrap_svc_probe"
))]
const INITD_SVC_FAILED_TRACE: &[u8] = b"NovaOS initd bootstrap svc failed\n";
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_virt_trace",
    feature = "bootstrap_trap_probe"
))]
const INITD_TRAP_BEGIN_TRACE: &[u8] = b"NovaOS initd bootstrap trap begin\n";
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_virt_trace",
    feature = "bootstrap_trap_probe"
))]
const INITD_TRAP_OK_TRACE: &[u8] = b"NovaOS initd bootstrap trap passed\n";
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_virt_trace",
    feature = "bootstrap_trap_probe"
))]
const INITD_TRAP_FAILED_TRACE: &[u8] = b"NovaOS initd bootstrap trap failed\n";
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_virt_trace"
))]
const INITD_CONTEXT_FAILED_TRACE: &[u8] = b"NovaOS initd bootstrap context invalid\n";
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_semihosting",
    not(feature = "qemu_virt_trace")
))]
const INITD_TRACE: &[u8] = b"NovaOS initd payload entered\n\0";
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_semihosting",
    not(feature = "qemu_virt_trace")
))]
const INITD_CONTEXT_OK_TRACE: &[u8] = b"NovaOS initd bootstrap context ready\n\0";
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_semihosting",
    not(feature = "qemu_virt_trace")
))]
const INITD_KERNEL_CALL_OK_TRACE: &[u8] = b"NovaOS initd bootstrap kernel call passed\n\0";
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_semihosting",
    not(feature = "qemu_virt_trace")
))]
const INITD_KERNEL_CALL_FAILED_TRACE: &[u8] = b"NovaOS initd bootstrap kernel call failed\n\0";
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_semihosting",
    not(feature = "qemu_virt_trace"),
    feature = "bootstrap_svc_probe"
))]
const INITD_SVC_BEGIN_TRACE: &[u8] = b"NovaOS initd bootstrap svc begin\n\0";
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_semihosting",
    not(feature = "qemu_virt_trace"),
    feature = "bootstrap_svc_probe"
))]
const INITD_SVC_OK_TRACE: &[u8] = b"NovaOS initd bootstrap svc passed\n\0";
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_semihosting",
    not(feature = "qemu_virt_trace"),
    feature = "bootstrap_svc_probe"
))]
const INITD_SVC_FAILED_TRACE: &[u8] = b"NovaOS initd bootstrap svc failed\n\0";
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_semihosting",
    not(feature = "qemu_virt_trace"),
    feature = "bootstrap_trap_probe"
))]
const INITD_TRAP_BEGIN_TRACE: &[u8] = b"NovaOS initd bootstrap trap begin\n\0";
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_semihosting",
    not(feature = "qemu_virt_trace"),
    feature = "bootstrap_trap_probe"
))]
const INITD_TRAP_OK_TRACE: &[u8] = b"NovaOS initd bootstrap trap passed\n\0";
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_semihosting",
    not(feature = "qemu_virt_trace"),
    feature = "bootstrap_trap_probe"
))]
const INITD_TRAP_FAILED_TRACE: &[u8] = b"NovaOS initd bootstrap trap failed\n\0";
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_semihosting",
    not(feature = "qemu_virt_trace")
))]
const INITD_CONTEXT_FAILED_TRACE: &[u8] = b"NovaOS initd bootstrap context invalid\n\0";

#[cfg(target_os = "none")]
#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {
        spin_loop();
    }
}

#[cfg(target_os = "none")]
#[unsafe(no_mangle)]
#[cfg(not(feature = "bootstrap_el0_probe"))]
pub extern "C" fn _start(context: *const NovaBootstrapTaskContextV1) -> ! {
    trace_initd_entry();
    trace_initd_context_result(context);
    maybe_trace_initd_kernel_call_result(context);
    maybe_trace_initd_svc_result(context);
    maybe_trace_initd_trap_result(context);
    loop {
        spin_loop();
    }
}

#[cfg(target_os = "none")]
#[unsafe(no_mangle)]
#[cfg(feature = "bootstrap_el0_probe")]
pub extern "C" fn _start(context: *const NovaBootstrapTaskContextV1) -> ! {
    maybe_trace_initd_svc_result(context);
    loop {
        spin_loop();
    }
}

#[cfg(not(target_os = "none"))]
fn main() {
    println!("{}", initd_identity());
}

pub fn initd_identity() -> &'static str {
    "NovaOS initd payload"
}

#[cfg(all(target_os = "none", not(feature = "bootstrap_el0_probe")))]
fn trace_initd_context_result(context: *const NovaBootstrapTaskContextV1) {
    if resolve_bootstrap_task_context(context).is_some() {
        trace_initd_context_success();
    } else {
        trace_initd_context_failure();
    }
}

#[cfg(all(target_os = "none", not(feature = "bootstrap_el0_probe")))]
fn maybe_trace_initd_kernel_call_result(context: *const NovaBootstrapTaskContextV1) {
    const TRACE_VALUE0: u64 = 0x494E_4954_444B_4341;
    const TRACE_VALUE1: u64 = 0x4E4F_5641_4B45_524E;

    let Some(_) = resolve_bootstrap_task_context(context) else {
        trace_initd_kernel_call_failure();
        return;
    };

    let result = bootstrap_trace(context, TRACE_VALUE0, TRACE_VALUE1);
    if result.status == NovaSyscallStatusV1::Ok as u32
        && result.value0 == TRACE_VALUE0
        && result.value1 == TRACE_VALUE1
    {
        trace_initd_kernel_call_success();
    } else {
        trace_initd_kernel_call_failure();
    }
}

#[cfg(all(
    target_os = "none",
    feature = "bootstrap_trap_probe",
    not(feature = "bootstrap_el0_probe")
))]
fn maybe_trace_initd_trap_result(context: *const NovaBootstrapTaskContextV1) {
    const TRACE_VALUE0: u64 = 0x494E_4954_4454_5241;
    const TRACE_VALUE1: u64 = 0x4E4F_5641_5452_4150;

    let Some(_) = resolve_bootstrap_task_context(context) else {
        trace_initd_trap_failure();
        return;
    };

    trace_initd_trap_begin();
    let result = bootstrap_trap_trace(TRACE_VALUE0, TRACE_VALUE1);
    if result.status == NovaSyscallStatusV1::Ok as u32
        && result.value0 == TRACE_VALUE0
        && result.value1 == TRACE_VALUE1
    {
        trace_initd_trap_success();
    } else {
        trace_initd_trap_failure();
    }
}

#[cfg(all(
    target_os = "none",
    not(feature = "bootstrap_trap_probe"),
    not(feature = "bootstrap_el0_probe")
))]
fn maybe_trace_initd_trap_result(_context: *const NovaBootstrapTaskContextV1) {}

#[cfg(all(
    target_os = "none",
    feature = "bootstrap_svc_probe",
    not(feature = "bootstrap_el0_probe")
))]
fn maybe_trace_initd_svc_result(context: *const NovaBootstrapTaskContextV1) {
    const TRACE_VALUE0: u64 = 0x494E_4954_4453_5643;
    const TRACE_VALUE1: u64 = 0x4E4F_5641_5356_4321;

    let Some(_) = resolve_bootstrap_task_context(context) else {
        trace_initd_svc_failure();
        return;
    };

    trace_initd_svc_begin();
    let result = trace(TRACE_VALUE0, TRACE_VALUE1);
    if result.status == NovaSyscallStatusV1::Ok as u32
        && result.value0 == TRACE_VALUE0
        && result.value1 == TRACE_VALUE1
    {
        trace_initd_svc_success();
    } else {
        trace_initd_svc_failure();
    }
}

#[cfg(all(
    target_os = "none",
    feature = "bootstrap_svc_probe",
    feature = "bootstrap_el0_probe"
))]
fn maybe_trace_initd_svc_result(_context: *const NovaBootstrapTaskContextV1) {
    const TRACE_VALUE0: u64 = 0x494E_4954_4453_5643;
    const TRACE_VALUE1: u64 = 0x4E4F_5641_5356_4321;

    let _ = trace(TRACE_VALUE0, TRACE_VALUE1);
}

#[cfg(all(target_os = "none", not(feature = "bootstrap_svc_probe")))]
fn maybe_trace_initd_svc_result(_context: *const NovaBootstrapTaskContextV1) {}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_virt_trace"
))]
#[allow(dead_code)]
fn trace_initd_entry() {
    qemu_uart_write(INITD_TRACE);
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_virt_trace"
))]
#[allow(dead_code)]
fn trace_initd_context_success() {
    qemu_uart_write(INITD_CONTEXT_OK_TRACE);
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_virt_trace"
))]
#[allow(dead_code)]
fn trace_initd_context_failure() {
    qemu_uart_write(INITD_CONTEXT_FAILED_TRACE);
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_virt_trace"
))]
#[allow(dead_code)]
fn trace_initd_kernel_call_success() {
    qemu_uart_write(INITD_KERNEL_CALL_OK_TRACE);
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_virt_trace"
))]
#[allow(dead_code)]
fn trace_initd_kernel_call_failure() {
    qemu_uart_write(INITD_KERNEL_CALL_FAILED_TRACE);
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_virt_trace",
    feature = "bootstrap_svc_probe"
))]
#[allow(dead_code)]
fn trace_initd_svc_begin() {
    qemu_uart_write(INITD_SVC_BEGIN_TRACE);
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_virt_trace",
    feature = "bootstrap_svc_probe"
))]
#[allow(dead_code)]
fn trace_initd_svc_success() {
    qemu_uart_write(INITD_SVC_OK_TRACE);
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_virt_trace",
    feature = "bootstrap_svc_probe"
))]
#[allow(dead_code)]
fn trace_initd_svc_failure() {
    qemu_uart_write(INITD_SVC_FAILED_TRACE);
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_virt_trace",
    feature = "bootstrap_trap_probe"
))]
#[allow(dead_code)]
fn trace_initd_trap_begin() {
    qemu_uart_write(INITD_TRAP_BEGIN_TRACE);
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_virt_trace",
    feature = "bootstrap_trap_probe"
))]
#[allow(dead_code)]
fn trace_initd_trap_success() {
    qemu_uart_write(INITD_TRAP_OK_TRACE);
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_virt_trace",
    feature = "bootstrap_trap_probe"
))]
#[allow(dead_code)]
fn trace_initd_trap_failure() {
    qemu_uart_write(INITD_TRAP_FAILED_TRACE);
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_semihosting",
    not(feature = "qemu_virt_trace")
))]
#[allow(dead_code)]
fn trace_initd_entry() {
    semihost_write0(INITD_TRACE);
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_semihosting",
    not(feature = "qemu_virt_trace")
))]
#[allow(dead_code)]
fn trace_initd_context_success() {
    semihost_write0(INITD_CONTEXT_OK_TRACE);
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_semihosting",
    not(feature = "qemu_virt_trace")
))]
#[allow(dead_code)]
fn trace_initd_context_failure() {
    semihost_write0(INITD_CONTEXT_FAILED_TRACE);
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_semihosting",
    not(feature = "qemu_virt_trace")
))]
#[allow(dead_code)]
fn trace_initd_kernel_call_success() {
    semihost_write0(INITD_KERNEL_CALL_OK_TRACE);
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_semihosting",
    not(feature = "qemu_virt_trace")
))]
#[allow(dead_code)]
fn trace_initd_kernel_call_failure() {
    semihost_write0(INITD_KERNEL_CALL_FAILED_TRACE);
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_semihosting",
    not(feature = "qemu_virt_trace"),
    feature = "bootstrap_svc_probe"
))]
#[allow(dead_code)]
fn trace_initd_svc_begin() {
    semihost_write0(INITD_SVC_BEGIN_TRACE);
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_semihosting",
    not(feature = "qemu_virt_trace"),
    feature = "bootstrap_svc_probe"
))]
#[allow(dead_code)]
fn trace_initd_svc_success() {
    semihost_write0(INITD_SVC_OK_TRACE);
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_semihosting",
    not(feature = "qemu_virt_trace"),
    feature = "bootstrap_svc_probe"
))]
#[allow(dead_code)]
fn trace_initd_svc_failure() {
    semihost_write0(INITD_SVC_FAILED_TRACE);
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_semihosting",
    not(feature = "qemu_virt_trace"),
    feature = "bootstrap_trap_probe"
))]
#[allow(dead_code)]
fn trace_initd_trap_begin() {
    semihost_write0(INITD_TRAP_BEGIN_TRACE);
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_semihosting",
    not(feature = "qemu_virt_trace"),
    feature = "bootstrap_trap_probe"
))]
#[allow(dead_code)]
fn trace_initd_trap_success() {
    semihost_write0(INITD_TRAP_OK_TRACE);
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_semihosting",
    not(feature = "qemu_virt_trace"),
    feature = "bootstrap_trap_probe"
))]
#[allow(dead_code)]
fn trace_initd_trap_failure() {
    semihost_write0(INITD_TRAP_FAILED_TRACE);
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
fn trace_initd_entry() {}

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
fn trace_initd_context_success() {}

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
fn trace_initd_context_failure() {}

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
fn trace_initd_kernel_call_success() {}

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
fn trace_initd_kernel_call_failure() {}

#[cfg(all(
    target_os = "none",
    not(any(
        all(
            target_arch = "aarch64",
            feature = "qemu_virt_trace",
            feature = "bootstrap_svc_probe"
        ),
        all(
            target_arch = "aarch64",
            feature = "qemu_semihosting",
            not(feature = "qemu_virt_trace"),
            feature = "bootstrap_svc_probe"
        )
    ))
))]
#[allow(dead_code)]
fn trace_initd_svc_begin() {}

#[cfg(all(
    target_os = "none",
    not(any(
        all(
            target_arch = "aarch64",
            feature = "qemu_virt_trace",
            feature = "bootstrap_svc_probe"
        ),
        all(
            target_arch = "aarch64",
            feature = "qemu_semihosting",
            not(feature = "qemu_virt_trace"),
            feature = "bootstrap_svc_probe"
        )
    ))
))]
#[allow(dead_code)]
fn trace_initd_svc_success() {}

#[cfg(all(
    target_os = "none",
    not(any(
        all(
            target_arch = "aarch64",
            feature = "qemu_virt_trace",
            feature = "bootstrap_svc_probe"
        ),
        all(
            target_arch = "aarch64",
            feature = "qemu_semihosting",
            not(feature = "qemu_virt_trace"),
            feature = "bootstrap_svc_probe"
        )
    ))
))]
#[allow(dead_code)]
fn trace_initd_svc_failure() {}

#[cfg(all(
    target_os = "none",
    not(any(
        all(
            target_arch = "aarch64",
            feature = "qemu_virt_trace",
            feature = "bootstrap_trap_probe"
        ),
        all(
            target_arch = "aarch64",
            feature = "qemu_semihosting",
            not(feature = "qemu_virt_trace"),
            feature = "bootstrap_trap_probe"
        )
    ))
))]
#[allow(dead_code)]
fn trace_initd_trap_begin() {}

#[cfg(all(
    target_os = "none",
    not(any(
        all(
            target_arch = "aarch64",
            feature = "qemu_virt_trace",
            feature = "bootstrap_trap_probe"
        ),
        all(
            target_arch = "aarch64",
            feature = "qemu_semihosting",
            not(feature = "qemu_virt_trace"),
            feature = "bootstrap_trap_probe"
        )
    ))
))]
#[allow(dead_code)]
fn trace_initd_trap_success() {}

#[cfg(all(
    target_os = "none",
    not(any(
        all(
            target_arch = "aarch64",
            feature = "qemu_virt_trace",
            feature = "bootstrap_trap_probe"
        ),
        all(
            target_arch = "aarch64",
            feature = "qemu_semihosting",
            not(feature = "qemu_virt_trace"),
            feature = "bootstrap_trap_probe"
        )
    ))
))]
#[allow(dead_code)]
fn trace_initd_trap_failure() {}

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
