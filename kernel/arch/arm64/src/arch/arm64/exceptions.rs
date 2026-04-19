#[cfg(all(target_os = "none", target_arch = "aarch64"))]
use core::arch::{asm, global_asm};
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
use core::sync::atomic::{AtomicU64, Ordering};

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
const EXCEPTION_VECTOR_TABLE_SIZE: usize = 2048;
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
const EXCEPTION_VECTOR_ALIGNMENT_MASK: u64 = EXCEPTION_VECTOR_TABLE_SIZE as u64 - 1;
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
const EXCEPTION_VECTOR_COPY_STORAGE_SIZE: usize =
    EXCEPTION_VECTOR_TABLE_SIZE + EXCEPTION_VECTOR_ALIGNMENT_MASK as usize;

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
static INSTALLED_EXCEPTION_VECTOR_BASE: AtomicU64 = AtomicU64::new(0);
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
static mut RUNTIME_EXCEPTION_VECTOR_COPY_STORAGE: [u8; EXCEPTION_VECTOR_COPY_STORAGE_SIZE] =
    [0; EXCEPTION_VECTOR_COPY_STORAGE_SIZE];
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_kernel_svc_probe"
))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct BootstrapExceptionReturnCapture {
    pub frame_x0: u64,
    pub frame_x1: u64,
    pub frame_x2: u64,
    pub restored_x0: u64,
    pub restored_x1: u64,
    pub restored_x2: u64,
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_kernel_svc_probe"
))]
impl BootstrapExceptionReturnCapture {
    const UNSET: u64 = u64::MAX;

    pub const fn unset() -> Self {
        Self {
            frame_x0: Self::UNSET,
            frame_x1: Self::UNSET,
            frame_x2: Self::UNSET,
            restored_x0: Self::UNSET,
            restored_x1: Self::UNSET,
            restored_x2: Self::UNSET,
        }
    }

    pub const fn is_recorded(self) -> bool {
        self.frame_x0 != Self::UNSET
    }
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_kernel_svc_probe"
))]
#[unsafe(no_mangle)]
static mut novaos_bootstrap_exception_return_capture: BootstrapExceptionReturnCapture =
    BootstrapExceptionReturnCapture::unset();

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_kernel_svc_probe"
))]
pub fn reset_bootstrap_exception_return_capture() {
    let capture = core::ptr::addr_of_mut!(novaos_bootstrap_exception_return_capture);
    unsafe {
        core::ptr::write_volatile(capture, BootstrapExceptionReturnCapture::unset());
    }
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_kernel_svc_probe"
))]
pub fn read_bootstrap_exception_return_capture() -> BootstrapExceptionReturnCapture {
    let capture = core::ptr::addr_of!(novaos_bootstrap_exception_return_capture);
    unsafe { core::ptr::read_volatile(capture) }
}

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
#[allow(unused_macros)]
#[cfg(feature = "bootstrap_kernel_svc_probe")]
macro_rules! bootstrap_exception_return_capture_asm {
    () => {
        r#"
    .macro nova_capture_exception_return
        adrp x23, novaos_bootstrap_exception_return_capture
        add x23, x23, :lo12:novaos_bootstrap_exception_return_capture
        ldr x24, [sp, #0]
        str x24, [x23, #0]
        ldr x24, [sp, #8]
        str x24, [x23, #8]
        ldr x24, [sp, #16]
        str x24, [x23, #16]
        str x0, [x23, #24]
        str x1, [x23, #32]
        str x2, [x23, #40]
    .endm
"#
    };
}

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
#[cfg(not(feature = "bootstrap_kernel_svc_probe"))]
macro_rules! bootstrap_exception_return_capture_asm {
    () => {
        r#"
    .macro nova_capture_exception_return
    .endm
"#
    };
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExceptionClass {
    Unknown = 0x00,
    Svc64 = 0x15,
    Brk64 = 0x3C,
    InstructionAbortLowerEl = 0x20,
    DataAbortLowerEl = 0x24,
}

impl ExceptionClass {
    pub const fn from_raw(raw: u8) -> Self {
        match raw {
            0x15 => Self::Svc64,
            0x3C => Self::Brk64,
            0x20 => Self::InstructionAbortLowerEl,
            0x24 => Self::DataAbortLowerEl,
            _ => Self::Unknown,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExceptionSyndrome {
    pub raw: u32,
    pub class: ExceptionClass,
    pub iss: u32,
}

impl ExceptionSyndrome {
    pub const ISS_MASK: u32 = (1 << 25) - 1;

    pub const fn from_esr(raw: u32) -> Self {
        let class = ExceptionClass::from_raw(((raw >> 26) & 0x3F) as u8);
        let iss = raw & Self::ISS_MASK;
        Self { raw, class, iss }
    }

    pub const fn svc_imm16(self) -> Option<u16> {
        if matches!(self.class, ExceptionClass::Svc64) {
            Some((self.iss & 0xFFFF) as u16)
        } else {
            None
        }
    }

    pub const fn brk_imm16(self) -> Option<u16> {
        if matches!(self.class, ExceptionClass::Brk64) {
            Some((self.iss & 0xFFFF) as u16)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExceptionVectors {
    pub base: u64,
    pub installed: bool,
}

impl ExceptionVectors {
    pub const fn placeholder() -> Self {
        Self {
            base: 0,
            installed: false,
        }
    }

    pub const fn from_base(base: u64) -> Self {
        Self {
            base,
            installed: true,
        }
    }

    #[cfg(all(target_os = "none", target_arch = "aarch64"))]
    pub fn runtime() -> Self {
        Self::from_base(core::ptr::addr_of!(__nova_exception_vectors) as u64)
    }

    #[cfg(all(target_os = "none", target_arch = "aarch64"))]
    pub fn installed_or_runtime() -> Self {
        let base = INSTALLED_EXCEPTION_VECTOR_BASE.load(Ordering::Relaxed);
        if base != 0 {
            Self::from_base(base)
        } else {
            Self::runtime()
        }
    }

    #[cfg(not(all(target_os = "none", target_arch = "aarch64")))]
    pub const fn runtime() -> Self {
        Self::placeholder()
    }

    #[cfg(all(target_os = "none", target_arch = "aarch64"))]
    pub unsafe fn install(&self) -> Self {
        let install_base = unsafe { install_exception_vector_base(self.base) };
        unsafe {
            asm!("msr vbar_el1, {}", in(reg) install_base, options(nostack, preserves_flags));
            asm!("isb", options(nostack, preserves_flags));
        }
        INSTALLED_EXCEPTION_VECTOR_BASE.store(install_base, Ordering::Relaxed);
        Self::from_base(install_base)
    }

    #[cfg(not(all(target_os = "none", target_arch = "aarch64")))]
    pub unsafe fn install(&self) -> Self {
        let _ = self;
        Self::placeholder()
    }
}

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
unsafe fn install_exception_vector_base(source_base: u64) -> u64 {
    if source_base & EXCEPTION_VECTOR_ALIGNMENT_MASK == 0 {
        return source_base;
    }

    let storage_ptr = core::ptr::addr_of_mut!(RUNTIME_EXCEPTION_VECTOR_COPY_STORAGE) as *mut u8;
    let aligned_copy_ptr = ((storage_ptr as usize + EXCEPTION_VECTOR_ALIGNMENT_MASK as usize)
        & !(EXCEPTION_VECTOR_ALIGNMENT_MASK as usize)) as *mut u8;
    unsafe {
        core::ptr::copy_nonoverlapping(
            source_base as *const u8,
            aligned_copy_ptr,
            EXCEPTION_VECTOR_TABLE_SIZE,
        );
    }
    sync_exception_vector_cache(aligned_copy_ptr as *const u8, EXCEPTION_VECTOR_TABLE_SIZE);
    aligned_copy_ptr as u64
}

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
fn sync_exception_vector_cache(ptr: *const u8, len: usize) {
    if ptr.is_null() || len == 0 {
        return;
    }

    let ctr_el0: u64;
    unsafe {
        asm!("mrs {}, ctr_el0", out(reg) ctr_el0);
    }

    let dcache_line = 4usize << ((ctr_el0 & 0xf) as usize);
    let icache_line = 4usize << (((ctr_el0 >> 16) & 0xf) as usize);
    let start = ptr as usize;
    let end = start + len;

    let mut line = start & !(dcache_line - 1);
    while line < end {
        unsafe {
            asm!("dc cvau, {}", in(reg) line);
        }
        line += dcache_line;
    }

    unsafe {
        asm!("dsb ish");
    }

    let mut line = start & !(icache_line - 1);
    while line < end {
        unsafe {
            asm!("ic ivau, {}", in(reg) line);
        }
        line += icache_line;
    }

    unsafe {
        asm!("dsb ish");
        asm!("isb");
    }
}

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
unsafe extern "C" {
    static __nova_exception_vectors: u8;
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    not(feature = "bootstrap_trap_vector_trace")
))]
global_asm!(concat!(
    bootstrap_exception_return_capture_asm!(),
    r#"
    .section .text.nova_exception_vectors, "ax"
    .balign 2048
    .global __nova_exception_vectors
__nova_exception_vectors:
    .macro nova_vector_slot target
        b \target
        .space 124
    .endm

    nova_vector_slot nova_exception_current_el_spx_sync
    nova_vector_slot nova_exception_default
    nova_vector_slot nova_exception_default
    nova_vector_slot nova_exception_default

    nova_vector_slot nova_exception_current_el_spx_sync
    nova_vector_slot nova_exception_default
    nova_vector_slot nova_exception_default
    nova_vector_slot nova_exception_default

    nova_vector_slot nova_exception_lower_el_aarch64_sync
    nova_vector_slot nova_exception_default
    nova_vector_slot nova_exception_default
    nova_vector_slot nova_exception_default

    nova_vector_slot nova_exception_default
    nova_vector_slot nova_exception_default
    nova_vector_slot nova_exception_default
    nova_vector_slot nova_exception_default

nova_exception_current_el_spx_sync:
    sub sp, sp, #272
    stp x0, x1, [sp, #0]
    stp x2, x3, [sp, #16]
    stp x4, x5, [sp, #32]
    stp x6, x7, [sp, #48]
    stp x8, x9, [sp, #64]
    stp x10, x11, [sp, #80]
    stp x12, x13, [sp, #96]
    stp x14, x15, [sp, #112]
    stp x16, x17, [sp, #128]
    stp x18, x19, [sp, #144]
    stp x20, x21, [sp, #160]
    stp x22, x23, [sp, #176]
    stp x24, x25, [sp, #192]
    stp x26, x27, [sp, #208]
    stp x28, x29, [sp, #224]
    str x30, [sp, #240]
    mrs x2, elr_el1
    str x2, [sp, #248]
    mrs x2, spsr_el1
    str x2, [sp, #256]
    mrs x0, esr_el1
    mov x1, sp
    bl novaos_current_el_sync_exception_handler
    cbz x0, nova_exception_default
    ldr x2, [sp, #248]
    msr elr_el1, x2
    ldr x2, [sp, #256]
    msr spsr_el1, x2
    ldp x0, x1, [sp, #0]
    ldp x2, x3, [sp, #16]
    nova_capture_exception_return
    ldp x4, x5, [sp, #32]
    ldp x6, x7, [sp, #48]
    ldp x8, x9, [sp, #64]
    ldp x10, x11, [sp, #80]
    ldp x12, x13, [sp, #96]
    ldp x14, x15, [sp, #112]
    ldp x16, x17, [sp, #128]
    ldp x18, x19, [sp, #144]
    ldp x20, x21, [sp, #160]
    ldp x22, x23, [sp, #176]
    ldp x24, x25, [sp, #192]
    ldp x26, x27, [sp, #208]
    ldp x28, x29, [sp, #224]
    ldr x30, [sp, #240]
    ldp x0, x1, [sp, #0]
    ldp x2, x3, [sp, #16]
    add sp, sp, #272
    eret

nova_exception_lower_el_aarch64_sync:
    sub sp, sp, #272
    stp x0, x1, [sp, #0]
    stp x2, x3, [sp, #16]
    stp x4, x5, [sp, #32]
    stp x6, x7, [sp, #48]
    stp x8, x9, [sp, #64]
    stp x10, x11, [sp, #80]
    stp x12, x13, [sp, #96]
    stp x14, x15, [sp, #112]
    stp x16, x17, [sp, #128]
    stp x18, x19, [sp, #144]
    stp x20, x21, [sp, #160]
    stp x22, x23, [sp, #176]
    stp x24, x25, [sp, #192]
    stp x26, x27, [sp, #208]
    stp x28, x29, [sp, #224]
    str x30, [sp, #240]
    mrs x2, elr_el1
    str x2, [sp, #248]
    mrs x2, spsr_el1
    str x2, [sp, #256]
    mrs x0, esr_el1
    mov x1, sp
    bl novaos_lower_el_aarch64_sync_exception_handler
    cbz x0, nova_exception_default
    ldr x2, [sp, #248]
    msr elr_el1, x2
    ldr x2, [sp, #256]
    msr spsr_el1, x2
    ldp x0, x1, [sp, #0]
    ldp x2, x3, [sp, #16]
    nova_capture_exception_return
    ldp x4, x5, [sp, #32]
    ldp x6, x7, [sp, #48]
    ldp x8, x9, [sp, #64]
    ldp x10, x11, [sp, #80]
    ldp x12, x13, [sp, #96]
    ldp x14, x15, [sp, #112]
    ldp x16, x17, [sp, #128]
    ldp x18, x19, [sp, #144]
    ldp x20, x21, [sp, #160]
    ldp x22, x23, [sp, #176]
    ldp x24, x25, [sp, #192]
    ldp x26, x27, [sp, #208]
    ldp x28, x29, [sp, #224]
    ldr x30, [sp, #240]
    ldp x0, x1, [sp, #0]
    ldp x2, x3, [sp, #16]
    add sp, sp, #272
    eret

nova_exception_default:
    wfe
    b nova_exception_default
"#
));

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_trap_vector_trace"
))]
global_asm!(concat!(
    bootstrap_exception_return_capture_asm!(),
    r#"
    .section .text.nova_exception_vectors, "ax"
    .balign 2048
    .global __nova_exception_vectors
__nova_exception_vectors:
    .macro nova_vector_slot target
        b \target
        .space 124
    .endm

    .macro nova_diag_uart_write byte_imm
999:
        ldr w17, [x16, #0x18]
        tst w17, #0x20
        b.ne 999b
        mov w17, #\byte_imm
        str w17, [x16]
    .endm

    .macro nova_diag_vector_prestack_marker
        sub sp, sp, #16
        stp x16, x17, [sp]
        movz x16, #0x0000
        movk x16, #0x0900, lsl #16
        nova_diag_uart_write 0x5b
        nova_diag_uart_write 0x56
        nova_diag_uart_write 0x50
        nova_diag_uart_write 0x5d
        nova_diag_uart_write 0x0a
        ldp x16, x17, [sp]
        add sp, sp, #16
    .endm

    nova_vector_slot nova_exception_current_el_spx_sync
    nova_vector_slot nova_exception_default
    nova_vector_slot nova_exception_default
    nova_vector_slot nova_exception_default

    nova_vector_slot nova_exception_current_el_spx_sync
    nova_vector_slot nova_exception_default
    nova_vector_slot nova_exception_default
    nova_vector_slot nova_exception_default

    nova_vector_slot nova_exception_lower_el_aarch64_sync
    nova_vector_slot nova_exception_default
    nova_vector_slot nova_exception_default
    nova_vector_slot nova_exception_default

    nova_vector_slot nova_exception_default
    nova_vector_slot nova_exception_default
    nova_vector_slot nova_exception_default
    nova_vector_slot nova_exception_default

nova_exception_current_el_spx_sync:
    nova_diag_vector_prestack_marker
    sub sp, sp, #272
    stp x0, x1, [sp, #0]
    stp x2, x3, [sp, #16]
    stp x4, x5, [sp, #32]
    stp x6, x7, [sp, #48]
    stp x8, x9, [sp, #64]
    stp x10, x11, [sp, #80]
    stp x12, x13, [sp, #96]
    stp x14, x15, [sp, #112]
    stp x16, x17, [sp, #128]
    stp x18, x19, [sp, #144]
    stp x20, x21, [sp, #160]
    stp x22, x23, [sp, #176]
    stp x24, x25, [sp, #192]
    stp x26, x27, [sp, #208]
    stp x28, x29, [sp, #224]
    str x30, [sp, #240]
    mrs x2, elr_el1
    str x2, [sp, #248]
    mrs x2, spsr_el1
    str x2, [sp, #256]
    mov x0, #1
    bl novaos_exception_vector_trace_marker
    mrs x0, esr_el1
    mov x1, sp
    bl novaos_current_el_sync_exception_handler
    mov x19, x0
    mov x0, #2
    bl novaos_exception_vector_trace_marker
    mov x0, x19
    cbz x0, nova_exception_default
    mov x0, #3
    bl novaos_exception_vector_trace_marker
    ldr x20, [sp, #0]
    cbz x20, 990f
    mov x0, #5
    bl novaos_exception_vector_trace_marker
    b 991f
990:
    mov x0, #6
    bl novaos_exception_vector_trace_marker
991:
    ldr x2, [sp, #248]
    msr elr_el1, x2
    ldr x2, [sp, #256]
    msr spsr_el1, x2
    ldp x0, x1, [sp, #0]
    ldp x2, x3, [sp, #16]
    nova_capture_exception_return
    ldp x4, x5, [sp, #32]
    ldp x6, x7, [sp, #48]
    ldp x8, x9, [sp, #64]
    ldp x10, x11, [sp, #80]
    ldp x12, x13, [sp, #96]
    ldp x14, x15, [sp, #112]
    ldp x16, x17, [sp, #128]
    ldp x18, x19, [sp, #144]
    ldp x20, x21, [sp, #160]
    ldp x22, x23, [sp, #176]
    ldp x24, x25, [sp, #192]
    ldp x26, x27, [sp, #208]
    ldp x28, x29, [sp, #224]
    ldr x30, [sp, #240]
    ldp x0, x1, [sp, #0]
    ldp x2, x3, [sp, #16]
    add sp, sp, #272
    eret

nova_exception_lower_el_aarch64_sync:
    nova_diag_vector_prestack_marker
    sub sp, sp, #272
    stp x0, x1, [sp, #0]
    stp x2, x3, [sp, #16]
    stp x4, x5, [sp, #32]
    stp x6, x7, [sp, #48]
    stp x8, x9, [sp, #64]
    stp x10, x11, [sp, #80]
    stp x12, x13, [sp, #96]
    stp x14, x15, [sp, #112]
    stp x16, x17, [sp, #128]
    stp x18, x19, [sp, #144]
    stp x20, x21, [sp, #160]
    stp x22, x23, [sp, #176]
    stp x24, x25, [sp, #192]
    stp x26, x27, [sp, #208]
    stp x28, x29, [sp, #224]
    str x30, [sp, #240]
    mrs x2, elr_el1
    str x2, [sp, #248]
    mrs x2, spsr_el1
    str x2, [sp, #256]
    mov x0, #1
    bl novaos_exception_vector_trace_marker
    mrs x0, esr_el1
    mov x1, sp
    bl novaos_lower_el_aarch64_sync_exception_handler
    mov x19, x0
    mov x0, #2
    bl novaos_exception_vector_trace_marker
    mov x0, x19
    cbz x0, nova_exception_default
    mov x0, #3
    bl novaos_exception_vector_trace_marker
    ldr x20, [sp, #0]
    cbz x20, 990f
    mov x0, #5
    bl novaos_exception_vector_trace_marker
    b 991f
990:
    mov x0, #6
    bl novaos_exception_vector_trace_marker
991:
    ldr x2, [sp, #248]
    msr elr_el1, x2
    ldr x2, [sp, #256]
    msr spsr_el1, x2
    ldp x0, x1, [sp, #0]
    ldp x2, x3, [sp, #16]
    nova_capture_exception_return
    ldp x4, x5, [sp, #32]
    ldp x6, x7, [sp, #48]
    ldp x8, x9, [sp, #64]
    ldp x10, x11, [sp, #80]
    ldp x12, x13, [sp, #96]
    ldp x14, x15, [sp, #112]
    ldp x16, x17, [sp, #128]
    ldp x18, x19, [sp, #144]
    ldp x20, x21, [sp, #160]
    ldp x22, x23, [sp, #176]
    ldp x24, x25, [sp, #192]
    ldp x26, x27, [sp, #208]
    ldp x28, x29, [sp, #224]
    ldr x30, [sp, #240]
    ldp x0, x1, [sp, #0]
    ldp x2, x3, [sp, #16]
    add sp, sp, #272
    eret

nova_exception_default:
    mov x0, #4
    bl novaos_exception_vector_trace_marker
    wfe
    b nova_exception_default
"#
));

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_trap_vector_trace"
))]
#[unsafe(no_mangle)]
extern "C" fn novaos_exception_vector_trace_marker(marker: u64) {
    match marker {
        1 => exception_trace_write(b"NovaOS bootstrap vector entered\n"),
        2 => exception_trace_write(b"NovaOS bootstrap vector handled\n"),
        3 => exception_trace_write(b"NovaOS bootstrap vector return\n"),
        4 => exception_trace_write(b"NovaOS bootstrap vector default\n"),
        5 => exception_trace_write(b"NovaOS bootstrap vector status slot nonzero\n"),
        6 => exception_trace_write(b"NovaOS bootstrap vector status slot zero\n"),
        _ => {}
    }
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_trap_vector_trace"
))]
fn exception_trace_write(message: &[u8]) {
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

#[cfg(test)]
mod tests {
    use super::{ExceptionClass, ExceptionSyndrome, ExceptionVectors};

    #[test]
    fn exception_syndrome_decodes_svc64() {
        let syndrome = ExceptionSyndrome::from_esr((ExceptionClass::Svc64 as u32) << 26 | 0x44);

        assert_eq!(syndrome.class, ExceptionClass::Svc64);
        assert_eq!(syndrome.svc_imm16(), Some(0x44));
    }

    #[test]
    fn exception_syndrome_reports_unknown_class() {
        let syndrome = ExceptionSyndrome::from_esr(0x3F << 26);

        assert_eq!(syndrome.class, ExceptionClass::Unknown);
        assert_eq!(syndrome.svc_imm16(), None);
    }

    #[test]
    fn exception_syndrome_decodes_brk64() {
        let syndrome = ExceptionSyndrome::from_esr((ExceptionClass::Brk64 as u32) << 26 | 0x77);

        assert_eq!(syndrome.class, ExceptionClass::Brk64);
        assert_eq!(syndrome.brk_imm16(), Some(0x77));
    }

    #[test]
    fn placeholder_vectors_report_uninstalled_state() {
        let vectors = ExceptionVectors::placeholder();

        assert_eq!(vectors.base, 0);
        assert!(!vectors.installed);
    }
}
