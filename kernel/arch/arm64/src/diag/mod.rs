use crate::arch::arm64::exceptions::ExceptionClass;
use crate::console::{self, ConsoleSink};
use crate::syscall::{
    Arm64SyscallFrame, SyscallDispatchState, dispatch_syscall,
    handle_lower_el_bootstrap_syscall_exception, handle_syscall_exception,
    install_bootstrap_syscall_state,
};
use nova_rt::{
    NovaInitCapsuleCapabilityV1, NovaSyscallNumberV1, NovaSyscallRequestV1, NovaSyscallStatusV1,
};

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
