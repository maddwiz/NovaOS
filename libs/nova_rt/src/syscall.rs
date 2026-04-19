use core::mem::size_of;

pub const NOVA_SYSCALL_ARG_COUNT: usize = 6;
pub const NOVA_BOOTSTRAP_TRAP_IMM16: u16 = 0x4E4F;

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NovaSyscallNumberV1 {
    Nop = 0,
    Trace = 1,
    Yield = 2,
    EndpointCall = 3,
    SharedMemoryMap = 4,
}

impl NovaSyscallNumberV1 {
    pub const fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            0 => Some(Self::Nop),
            1 => Some(Self::Trace),
            2 => Some(Self::Yield),
            3 => Some(Self::EndpointCall),
            4 => Some(Self::SharedMemoryMap),
            _ => None,
        }
    }
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NovaSyscallStatusV1 {
    Ok = 0,
    Unknown = 1,
    Unsupported = 2,
    Denied = 3,
    InvalidArgs = 4,
}

impl NovaSyscallStatusV1 {
    pub const fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            0 => Some(Self::Ok),
            1 => Some(Self::Unknown),
            2 => Some(Self::Unsupported),
            3 => Some(Self::Denied),
            4 => Some(Self::InvalidArgs),
            _ => None,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NovaSyscallRequestV1 {
    pub number: u32,
    pub flags: u32,
    pub args: [u64; NOVA_SYSCALL_ARG_COUNT],
}

impl NovaSyscallRequestV1 {
    pub const fn new(
        number: NovaSyscallNumberV1,
        flags: u32,
        args: [u64; NOVA_SYSCALL_ARG_COUNT],
    ) -> Self {
        Self {
            number: number as u32,
            flags,
            args,
        }
    }
}

pub const fn trace_request(value0: u64, value1: u64) -> NovaSyscallRequestV1 {
    NovaSyscallRequestV1::new(NovaSyscallNumberV1::Trace, 0, [value0, value1, 0, 0, 0, 0])
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NovaSyscallResultV1 {
    pub status: u32,
    pub flags: u32,
    pub value0: u64,
    pub value1: u64,
}

impl NovaSyscallResultV1 {
    pub const fn ok(value0: u64, value1: u64) -> Self {
        Self {
            status: NovaSyscallStatusV1::Ok as u32,
            flags: 0,
            value0,
            value1,
        }
    }

    pub const fn unknown() -> Self {
        Self {
            status: NovaSyscallStatusV1::Unknown as u32,
            flags: 0,
            value0: 0,
            value1: 0,
        }
    }

    pub const fn unsupported() -> Self {
        Self {
            status: NovaSyscallStatusV1::Unsupported as u32,
            flags: 0,
            value0: 0,
            value1: 0,
        }
    }

    pub const fn denied() -> Self {
        Self {
            status: NovaSyscallStatusV1::Denied as u32,
            flags: 0,
            value0: 0,
            value1: 0,
        }
    }

    pub const fn invalid_args() -> Self {
        Self {
            status: NovaSyscallStatusV1::InvalidArgs as u32,
            flags: 0,
            value0: 0,
            value1: 0,
        }
    }

    pub const fn is_success(&self) -> bool {
        self.status == NovaSyscallStatusV1::Ok as u32
    }
}

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
pub fn syscall(request: NovaSyscallRequestV1) -> NovaSyscallResultV1 {
    let mut raw = [0u64; 3];
    raw_syscall_result_from_svc(&mut raw, request);
    decode_raw_syscall_result(raw)
}

#[cfg(not(all(target_os = "none", target_arch = "aarch64")))]
pub fn syscall(_request: NovaSyscallRequestV1) -> NovaSyscallResultV1 {
    NovaSyscallResultV1::unsupported()
}

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
pub fn bootstrap_trap(request: NovaSyscallRequestV1) -> NovaSyscallResultV1 {
    let mut raw = [0u64; 3];
    raw_syscall_result_from_bootstrap_trap(&mut raw, request);
    decode_raw_syscall_result(raw)
}

#[cfg(not(all(target_os = "none", target_arch = "aarch64")))]
pub fn bootstrap_trap(_request: NovaSyscallRequestV1) -> NovaSyscallResultV1 {
    NovaSyscallResultV1::unsupported()
}

pub fn trace(value0: u64, value1: u64) -> NovaSyscallResultV1 {
    syscall(trace_request(value0, value1))
}

pub fn bootstrap_trap_trace(value0: u64, value1: u64) -> NovaSyscallResultV1 {
    bootstrap_trap(trace_request(value0, value1))
}

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
fn decode_raw_syscall_result(raw: [u64; 3]) -> NovaSyscallResultV1 {
    NovaSyscallResultV1 {
        status: NovaSyscallStatusV1::from_raw(raw[0] as u32).unwrap_or(NovaSyscallStatusV1::Unknown)
            as u32,
        flags: 0,
        value0: raw[1],
        value1: raw[2],
    }
}

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
fn raw_syscall_result_from_svc(raw: &mut [u64; 3], request: NovaSyscallRequestV1) {
    unsafe {
        core::arch::asm!(
            "mov x0, x10",
            "mov x1, x11",
            "mov x2, x12",
            "mov x3, x13",
            "mov x4, x14",
            "mov x5, x15",
            "mov x6, x16",
            "mov x7, xzr",
            "mov x8, x17",
            "svc #0",
            "stp x0, x1, [x9]",
            "str x2, [x9, #16]",
            in("x9") raw.as_mut_ptr(),
            in("x10") request.args[0],
            in("x11") request.args[1],
            in("x12") request.args[2],
            in("x13") request.args[3],
            in("x14") request.args[4],
            in("x15") request.args[5],
            in("x16") request.flags as u64,
            in("x17") request.number as u64,
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

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
fn raw_syscall_result_from_bootstrap_trap(raw: &mut [u64; 3], request: NovaSyscallRequestV1) {
    unsafe {
        core::arch::asm!(
            "mov x0, x10",
            "mov x1, x11",
            "mov x2, x12",
            "mov x3, x13",
            "mov x4, x14",
            "mov x5, x15",
            "mov x6, x16",
            "mov x7, xzr",
            "mov x8, x17",
            "brk #0x4e4f",
            "stp x0, x1, [x9]",
            "str x2, [x9, #16]",
            in("x9") raw.as_mut_ptr(),
            in("x10") request.args[0],
            in("x11") request.args[1],
            in("x12") request.args[2],
            in("x13") request.args[3],
            in("x14") request.args[4],
            in("x15") request.args[5],
            in("x16") request.flags as u64,
            in("x17") request.number as u64,
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

const _: [(); 56] = [(); size_of::<NovaSyscallRequestV1>()];
const _: [(); 24] = [(); size_of::<NovaSyscallResultV1>()];

#[cfg(test)]
mod tests {
    use super::{
        NOVA_BOOTSTRAP_TRAP_IMM16, NOVA_SYSCALL_ARG_COUNT, NovaSyscallNumberV1,
        NovaSyscallRequestV1, NovaSyscallResultV1, NovaSyscallStatusV1, bootstrap_trap_trace,
        trace, trace_request,
    };
    use core::mem::{offset_of, size_of};

    #[test]
    fn syscall_layout_matches_c_header() {
        assert_eq!(NOVA_SYSCALL_ARG_COUNT, 6);
        assert_eq!(size_of::<NovaSyscallRequestV1>(), 56);
        assert_eq!(size_of::<NovaSyscallResultV1>(), 24);
        assert_eq!(offset_of!(NovaSyscallRequestV1, args), 8);
        assert_eq!(offset_of!(NovaSyscallResultV1, value0), 8);
    }

    #[test]
    fn syscall_enums_match_public_abi_values() {
        assert_eq!(NovaSyscallNumberV1::Nop as u32, 0);
        assert_eq!(NovaSyscallNumberV1::Trace as u32, 1);
        assert_eq!(NovaSyscallNumberV1::Yield as u32, 2);
        assert_eq!(NovaSyscallNumberV1::EndpointCall as u32, 3);
        assert_eq!(NovaSyscallNumberV1::SharedMemoryMap as u32, 4);
        assert_eq!(NovaSyscallStatusV1::Ok as u32, 0);
        assert_eq!(NovaSyscallStatusV1::Denied as u32, 3);
    }

    #[test]
    fn syscall_result_helpers_report_expected_status() {
        assert!(NovaSyscallResultV1::ok(1, 2).is_success());
        assert!(!NovaSyscallResultV1::unsupported().is_success());
        assert_eq!(
            NovaSyscallStatusV1::from_raw(NovaSyscallResultV1::denied().status),
            Some(NovaSyscallStatusV1::Denied)
        );
    }

    #[test]
    fn trace_request_uses_trace_number_and_args() {
        let request = trace_request(0xAA, 0xBB);

        assert_eq!(request.number, NovaSyscallNumberV1::Trace as u32);
        assert_eq!(request.args[0], 0xAA);
        assert_eq!(request.args[1], 0xBB);
    }

    #[test]
    fn host_trace_wrapper_returns_unsupported() {
        let result = trace(0xAA, 0xBB);

        assert_eq!(result.status, NovaSyscallStatusV1::Unsupported as u32);
    }

    #[test]
    fn bootstrap_trap_constant_matches_expected_marker() {
        assert_eq!(NOVA_BOOTSTRAP_TRAP_IMM16, 0x4E4F);
        assert_eq!(
            bootstrap_trap_trace(0xAA, 0xBB).status,
            NovaSyscallStatusV1::Unsupported as u32
        );
    }
}
