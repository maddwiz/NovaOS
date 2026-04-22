use crate::arch::arm64::exceptions::ExceptionClass;
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    any(
        feature = "bootstrap_kernel_svc_probe",
        feature = "bootstrap_pretransfer_svc_probe",
        feature = "bootstrap_trap_vector_trace"
    )
))]
use crate::arch::arm64::exceptions::ExceptionVectors;
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_kernel_svc_probe"
))]
use crate::arch::arm64::exceptions::{
    BootstrapExceptionReturnCapture, read_bootstrap_exception_return_capture,
    reset_bootstrap_exception_return_capture,
};
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    any(
        feature = "bootstrap_kernel_svc_probe",
        feature = "bootstrap_pretransfer_svc_probe"
    )
))]
use crate::console::TraceConsole;
use crate::console::{self, ConsoleSink};
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    any(
        feature = "bootstrap_kernel_svc_probe",
        feature = "bootstrap_pretransfer_svc_probe",
        feature = "bootstrap_trap_vector_trace"
    )
))]
use crate::el::read_runtime_vbar_el1;
use crate::syscall::{
    Arm64SyscallFrame, SyscallDispatchState, dispatch_syscall,
    handle_lower_el_bootstrap_syscall_exception, handle_syscall_exception,
    install_bootstrap_syscall_state,
};
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_kernel_svc_probe"
))]
use nova_rt::syscall::trace;
use nova_rt::{
    NovaInitCapsuleCapabilityV1, NovaSyscallNumberV1, NovaSyscallRequestV1, NovaSyscallStatusV1,
};

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

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    any(
        feature = "bootstrap_kernel_svc_probe",
        feature = "bootstrap_pretransfer_svc_probe"
    )
))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RuntimeExceptionProbeState {
    current_el: u64,
    spsel: u64,
    vbar_el1: u64,
    expected_vbar_el1: u64,
}

pub(crate) fn run_syscall_probe<C: ConsoleSink>(console: &mut C, state: SyscallDispatchState) {
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

pub(crate) fn run_lower_el_bootstrap_svc_dry_run<C: ConsoleSink>(
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

pub(crate) fn write_hex_u64<C: ConsoleSink>(console: &mut C, value: u64) {
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
pub(crate) fn trace_kernel_stage0_marker(message: &[u8]) {
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
pub(crate) fn trace_kernel_stage0_marker(_message: &[u8]) {}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    any(
        feature = "bootstrap_kernel_svc_probe",
        feature = "bootstrap_pretransfer_svc_probe"
    )
))]
pub(crate) fn read_runtime_exception_probe_state() -> RuntimeExceptionProbeState {
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
pub(crate) fn log_runtime_exception_probe_state(label: &str, state: RuntimeExceptionProbeState) {
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
pub(crate) fn log_bootstrap_exception_install_status(
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
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_kernel_svc_probe"
))]
static mut BOOTSTRAP_KERNEL_SVC_CALLER_CAPTURE: BootstrapKernelSvcCallerCapture =
    BootstrapKernelSvcCallerCapture::unset();

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
pub(crate) fn run_bootstrap_kernel_svc_probe() -> ! {
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

    crate::panic::halt();
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
