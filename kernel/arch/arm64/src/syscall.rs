use crate::arch::arm64::exceptions::{ExceptionClass, ExceptionSyndrome};
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
use crate::console::TraceConsole;
use crate::console::{ConsoleSink, LogLevel};
use nova_rt::{
    NOVA_BOOTSTRAP_TRAP_IMM16, NOVA_INIT_CAPSULE_SERVICE_NAME_LEN, NOVA_SYSCALL_ARG_COUNT,
    NovaInitCapsuleCapabilityV1, NovaSyscallNumberV1, NovaSyscallRequestV1, NovaSyscallResultV1,
    decode_init_capsule_service_name,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BootstrapTaskState {
    pub bootstrap_capabilities: u64,
    pub endpoint_slots: u32,
    pub shared_memory_regions: u32,
}

impl BootstrapTaskState {
    pub const fn new(
        bootstrap_capabilities: u64,
        endpoint_slots: u32,
        shared_memory_regions: u32,
    ) -> Self {
        Self {
            bootstrap_capabilities,
            endpoint_slots,
            shared_memory_regions,
        }
    }

    pub const fn has_bootstrap_capability(&self, capability: NovaInitCapsuleCapabilityV1) -> bool {
        (self.bootstrap_capabilities & capability as u64) != 0
    }

    pub const fn contains_endpoint_slot(&self, slot: u64) -> bool {
        slot < self.endpoint_slots as u64
    }

    pub const fn contains_shared_memory_region(&self, region: u64) -> bool {
        region < self.shared_memory_regions as u64
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CurrentTaskState {
    pub service_name: [u8; NOVA_INIT_CAPSULE_SERVICE_NAME_LEN],
    pub bootstrap: BootstrapTaskState,
}

impl CurrentTaskState {
    pub const fn new(
        service_name: [u8; NOVA_INIT_CAPSULE_SERVICE_NAME_LEN],
        bootstrap: BootstrapTaskState,
    ) -> Self {
        Self {
            service_name,
            bootstrap,
        }
    }

    pub fn service_name(&self) -> &str {
        decode_init_capsule_service_name(&self.service_name)
            .expect("current task service name must stay valid")
    }

    pub const fn bootstrap(&self) -> BootstrapTaskState {
        self.bootstrap
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SyscallDispatchState {
    pub current_task: Option<CurrentTaskState>,
    pub endpoints_ready: bool,
    pub shared_memory_ready: bool,
}

impl SyscallDispatchState {
    pub const fn scaffold() -> Self {
        Self {
            current_task: None,
            endpoints_ready: false,
            shared_memory_ready: false,
        }
    }

    pub const fn bootstrap(
        current_task: CurrentTaskState,
        endpoints_ready: bool,
        shared_memory_ready: bool,
    ) -> Self {
        Self {
            current_task: Some(current_task),
            endpoints_ready,
            shared_memory_ready,
        }
    }

    pub fn has_bootstrap_capability(&self, capability: NovaInitCapsuleCapabilityV1) -> bool {
        self.current_task
            .is_some_and(|task| task.bootstrap().has_bootstrap_capability(capability))
    }

    pub fn contains_endpoint_slot(&self, slot: u64) -> bool {
        self.current_task
            .is_some_and(|task| task.bootstrap().contains_endpoint_slot(slot))
    }

    pub fn contains_shared_memory_region(&self, region: u64) -> bool {
        self.current_task
            .is_some_and(|task| task.bootstrap().contains_shared_memory_region(region))
    }

    pub fn current_task_service_name(&self) -> Option<&str> {
        self.current_task
            .as_ref()
            .map(CurrentTaskState::service_name)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct Arm64SyscallFrame {
    pub registers: [u64; 31],
    pub elr: u64,
    pub spsr: u64,
}

impl Arm64SyscallFrame {
    pub const FLAG_REGISTER: usize = 6;
    pub const RESERVED_REGISTER: usize = 7;
    pub const NUMBER_REGISTER: usize = 8;
    pub const STATUS_REGISTER: usize = 0;
    pub const VALUE0_REGISTER: usize = 1;
    pub const VALUE1_REGISTER: usize = 2;
    pub const SYSCALL_INSTRUCTION_LEN: u64 = 4;

    pub const fn empty() -> Self {
        Self {
            registers: [0; 31],
            elr: 0,
            spsr: 0,
        }
    }

    pub fn from_request(request: NovaSyscallRequestV1) -> Self {
        let mut frame = Self::empty();
        frame.registers[..NOVA_SYSCALL_ARG_COUNT].copy_from_slice(&request.args);
        frame.registers[Self::FLAG_REGISTER] = request.flags as u64;
        frame.registers[Self::NUMBER_REGISTER] = request.number as u64;
        frame
    }

    pub fn request(&self) -> NovaSyscallRequestV1 {
        let mut args = [0; NOVA_SYSCALL_ARG_COUNT];
        args.copy_from_slice(&self.registers[..NOVA_SYSCALL_ARG_COUNT]);

        NovaSyscallRequestV1 {
            number: self.registers[Self::NUMBER_REGISTER] as u32,
            flags: self.registers[Self::FLAG_REGISTER] as u32,
            args,
        }
    }

    pub fn apply_result(&mut self, result: NovaSyscallResultV1) {
        self.apply_result_with_return_elr(
            result,
            self.elr.wrapping_add(Self::SYSCALL_INSTRUCTION_LEN),
        );
    }

    pub fn apply_result_without_elr_advance(&mut self, result: NovaSyscallResultV1) {
        self.apply_result_with_return_elr(result, self.elr);
    }

    fn apply_result_with_return_elr(&mut self, result: NovaSyscallResultV1, return_elr: u64) {
        #[cfg(all(target_os = "none", target_arch = "aarch64"))]
        unsafe {
            // This frame is restored by external vector assembly immediately after
            // the handler returns. Use explicit stores at the exact ABI offsets.
            core::arch::asm!(
                "str {status}, [{frame}]",
                "str {value0}, [{frame}, #8]",
                "str {value1}, [{frame}, #16]",
                "str {next_elr}, [{frame}, #248]",
                frame = in(reg) self as *mut Self,
                status = in(reg) result.status as u64,
                value0 = in(reg) result.value0,
                value1 = in(reg) result.value1,
                next_elr = in(reg) return_elr,
                options(nostack, preserves_flags),
            );
        }

        #[cfg(not(all(target_os = "none", target_arch = "aarch64")))]
        {
            self.registers[Self::STATUS_REGISTER] = result.status as u64;
            self.registers[Self::VALUE0_REGISTER] = result.value0;
            self.registers[Self::VALUE1_REGISTER] = result.value1;
            self.elr = return_elr;
        }
    }
}

static mut BOOTSTRAP_SYSCALL_DISPATCH_STATE: SyscallDispatchState = SyscallDispatchState {
    current_task: None,
    endpoints_ready: false,
    shared_memory_ready: false,
};

pub fn install_bootstrap_syscall_state(state: SyscallDispatchState) {
    unsafe {
        BOOTSTRAP_SYSCALL_DISPATCH_STATE = state;
    }
}

pub fn bootstrap_syscall_state() -> SyscallDispatchState {
    unsafe { BOOTSTRAP_SYSCALL_DISPATCH_STATE }
}

pub fn handle_syscall_exception<C: ConsoleSink>(
    esr: u32,
    frame: &mut Arm64SyscallFrame,
    state: &SyscallDispatchState,
    console: &mut C,
) -> bool {
    let syndrome = ExceptionSyndrome::from_esr(esr);
    if syndrome.class != ExceptionClass::Svc64 {
        return false;
    }

    let result = dispatch_syscall(state, frame.request(), console);
    frame.apply_result(result);
    true
}

pub fn handle_bootstrap_syscall_exception<C: ConsoleSink>(
    esr: u32,
    frame: &mut Arm64SyscallFrame,
    console: &mut C,
) -> bool {
    let state = bootstrap_syscall_state();
    let syndrome = ExceptionSyndrome::from_esr(esr);
    match syndrome.class {
        ExceptionClass::Svc64 => {
            if let Some(service_name) = state.current_task_service_name() {
                console.write_str("[info] bootstrap live svc from ");
                console.write_line(service_name);
            } else {
                console.log(LogLevel::Info, "bootstrap live svc");
            }

            let result = dispatch_syscall(&state, frame.request(), console);
            // Current-EL SVC frames in this bootstrap lane already report the
            // preferred post-SVC return address. Advancing here skips the first
            // caller instruction after `svc`.
            frame.apply_result_without_elr_advance(result);
            #[cfg(feature = "bootstrap_trap_vector_trace")]
            log_live_exception_frame(console, "bootstrap live svc frame", frame);
            true
        }
        ExceptionClass::Brk64 => {
            if syndrome.brk_imm16() != Some(NOVA_BOOTSTRAP_TRAP_IMM16) {
                console.log(LogLevel::Warn, "bootstrap live trap immediate mismatch");
                return false;
            }

            if let Some(service_name) = state.current_task_service_name() {
                console.write_str("[info] bootstrap live trap from ");
                console.write_line(service_name);
            } else {
                console.log(LogLevel::Info, "bootstrap live trap");
            }

            let result = dispatch_syscall(&state, frame.request(), console);
            frame.apply_result(result);
            #[cfg(feature = "bootstrap_trap_vector_trace")]
            log_live_exception_frame(console, "bootstrap live trap frame", frame);
            true
        }
        ExceptionClass::InstructionAbortLowerEl => {
            console.log(
                LogLevel::Warn,
                "bootstrap sync exception instruction_abort_lower_el",
            );
            false
        }
        ExceptionClass::DataAbortLowerEl => {
            console.log(
                LogLevel::Warn,
                "bootstrap sync exception data_abort_lower_el",
            );
            false
        }
        ExceptionClass::Unknown => {
            console.log(LogLevel::Warn, "bootstrap sync exception unknown");
            false
        }
    }
}

pub fn handle_lower_el_bootstrap_syscall_exception<C: ConsoleSink>(
    esr: u32,
    frame: &mut Arm64SyscallFrame,
    console: &mut C,
) -> bool {
    let state = bootstrap_syscall_state();
    let syndrome = ExceptionSyndrome::from_esr(esr);
    if syndrome.class != ExceptionClass::Svc64 {
        return false;
    }

    if let Some(service_name) = state.current_task_service_name() {
        console.write_str("[info] bootstrap lower-el svc from ");
        console.write_line(service_name);
    } else {
        console.log(LogLevel::Info, "bootstrap lower-el svc");
    }

    let result = dispatch_syscall(&state, frame.request(), console);
    frame.apply_result(result);
    true
}

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
#[unsafe(no_mangle)]
extern "C" fn novaos_current_el_sync_exception_handler(
    esr: u64,
    frame: *mut Arm64SyscallFrame,
) -> u64 {
    let Some(frame) = (unsafe { frame.as_mut() }) else {
        return 0;
    };

    let mut console = TraceConsole::new();
    u64::from(handle_bootstrap_syscall_exception(
        esr as u32,
        frame,
        &mut console,
    ))
}

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
#[unsafe(no_mangle)]
extern "C" fn novaos_lower_el_aarch64_sync_exception_handler(
    esr: u64,
    frame: *mut Arm64SyscallFrame,
) -> u64 {
    let Some(frame) = (unsafe { frame.as_mut() }) else {
        return 0;
    };

    let mut console = TraceConsole::new();
    u64::from(handle_lower_el_bootstrap_syscall_exception(
        esr as u32,
        frame,
        &mut console,
    ))
}

#[cfg(feature = "bootstrap_trap_vector_trace")]
fn log_live_exception_frame<C: ConsoleSink>(
    console: &mut C,
    label: &str,
    frame: &Arm64SyscallFrame,
) {
    console.write_str("[info] ");
    console.write_str(label);
    console.write_str(" status ");
    write_hex_u64(console, frame.registers[Arm64SyscallFrame::STATUS_REGISTER]);
    console.write_str(" value0 ");
    write_hex_u64(console, frame.registers[Arm64SyscallFrame::VALUE0_REGISTER]);
    console.write_str(" value1 ");
    write_hex_u64(console, frame.registers[Arm64SyscallFrame::VALUE1_REGISTER]);
    console.write_str("\n");
}

#[cfg(feature = "bootstrap_trap_vector_trace")]
fn write_hex_u64<C: ConsoleSink>(console: &mut C, value: u64) {
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

pub fn dispatch_syscall<C: ConsoleSink>(
    state: &SyscallDispatchState,
    request: NovaSyscallRequestV1,
    console: &mut C,
) -> NovaSyscallResultV1 {
    let Some(number) = NovaSyscallNumberV1::from_raw(request.number) else {
        return NovaSyscallResultV1::unknown();
    };

    match number {
        NovaSyscallNumberV1::Nop => NovaSyscallResultV1::ok(0, 0),
        NovaSyscallNumberV1::Trace => {
            if !state.has_bootstrap_capability(NovaInitCapsuleCapabilityV1::BootLog) {
                return NovaSyscallResultV1::denied();
            }

            if let Some(service_name) = state.current_task_service_name() {
                console.write_str("[info] syscall trace request from ");
                console.write_line(service_name);
            } else {
                console.log(LogLevel::Info, "syscall trace request");
            }
            NovaSyscallResultV1::ok(request.args[0], request.args[1])
        }
        NovaSyscallNumberV1::Yield => {
            if !state.has_bootstrap_capability(NovaInitCapsuleCapabilityV1::Yield) {
                return NovaSyscallResultV1::denied();
            }

            NovaSyscallResultV1::unsupported()
        }
        NovaSyscallNumberV1::EndpointCall => {
            if !state.has_bootstrap_capability(NovaInitCapsuleCapabilityV1::EndpointBootstrap) {
                return NovaSyscallResultV1::denied();
            }
            if !state.contains_endpoint_slot(request.args[0]) {
                return NovaSyscallResultV1::invalid_args();
            }
            if !state.endpoints_ready {
                return NovaSyscallResultV1::unsupported();
            }

            if let Some(service_name) = state.current_task_service_name() {
                console.write_str("[info] bootstrap endpoint call from ");
                console.write_line(service_name);
            } else {
                console.log(LogLevel::Info, "bootstrap endpoint call");
            }

            NovaSyscallResultV1::ok(request.args[0], request.args[1])
        }
        NovaSyscallNumberV1::SharedMemoryMap => {
            if !state.has_bootstrap_capability(NovaInitCapsuleCapabilityV1::SharedMemoryBootstrap) {
                return NovaSyscallResultV1::denied();
            }
            if !state.contains_shared_memory_region(request.args[0]) {
                return NovaSyscallResultV1::invalid_args();
            }
            if !state.shared_memory_ready {
                return NovaSyscallResultV1::unsupported();
            }

            if let Some(service_name) = state.current_task_service_name() {
                console.write_str("[info] bootstrap shared memory map from ");
                console.write_line(service_name);
            } else {
                console.log(LogLevel::Info, "bootstrap shared memory map");
            }

            NovaSyscallResultV1::ok(request.args[0], request.args[1])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Arm64SyscallFrame, BootstrapTaskState, CurrentTaskState, SyscallDispatchState,
        bootstrap_syscall_state, dispatch_syscall, handle_bootstrap_syscall_exception,
        handle_lower_el_bootstrap_syscall_exception, handle_syscall_exception,
        install_bootstrap_syscall_state,
    };
    use crate::arch::arm64::exceptions::ExceptionClass;
    use crate::console::ConsoleSink;
    use nova_rt::{
        NOVA_BOOTSTRAP_TRAP_IMM16, NovaInitCapsuleCapabilityV1, NovaSyscallNumberV1,
        NovaSyscallRequestV1, NovaSyscallStatusV1,
    };

    #[test]
    fn dispatch_nop_returns_ok() {
        let mut console = RecordingConsole::new();
        let result = dispatch_syscall(
            &SyscallDispatchState::scaffold(),
            NovaSyscallRequestV1::new(NovaSyscallNumberV1::Nop, 0, [0; 6]),
            &mut console,
        );

        assert_eq!(result.status, NovaSyscallStatusV1::Ok as u32);
        assert_eq!(result.value0, 0);
        assert_eq!(console.as_str(), "");
    }

    #[test]
    fn trace_requires_boot_log_capability() {
        let mut console = RecordingConsole::new();
        let result = dispatch_syscall(
            &SyscallDispatchState::scaffold(),
            NovaSyscallRequestV1::new(NovaSyscallNumberV1::Trace, 0, [0xAA, 0xBB, 0, 0, 0, 0]),
            &mut console,
        );

        assert_eq!(result.status, NovaSyscallStatusV1::Denied as u32);
        assert_eq!(console.as_str(), "");
    }

    #[test]
    fn trace_allows_boot_log_capability() {
        let mut console = RecordingConsole::new();
        let result = dispatch_syscall(
            &SyscallDispatchState::bootstrap(
                initd_task(NovaInitCapsuleCapabilityV1::BootLog as u64, 0, 0),
                false,
                false,
            ),
            NovaSyscallRequestV1::new(NovaSyscallNumberV1::Trace, 0, [0xAA, 0xBB, 0, 0, 0, 0]),
            &mut console,
        );

        assert_eq!(result.status, NovaSyscallStatusV1::Ok as u32);
        assert_eq!(result.value0, 0xAA);
        assert_eq!(result.value1, 0xBB);
        assert!(
            console
                .as_str()
                .contains("syscall trace request from initd")
        );
    }

    #[test]
    fn handle_syscall_exception_round_trips_trace_request() {
        let request =
            NovaSyscallRequestV1::new(NovaSyscallNumberV1::Trace, 0, [0xAA, 0xBB, 0, 0, 0, 0]);
        let mut frame = Arm64SyscallFrame::from_request(request);
        let mut console = RecordingConsole::new();
        frame.elr = 0x4000;

        let handled = handle_syscall_exception(
            (ExceptionClass::Svc64 as u32) << 26,
            &mut frame,
            &SyscallDispatchState::bootstrap(
                initd_task(NovaInitCapsuleCapabilityV1::BootLog as u64, 0, 0),
                false,
                false,
            ),
            &mut console,
        );

        assert!(handled);
        assert_eq!(
            frame.registers[Arm64SyscallFrame::STATUS_REGISTER],
            NovaSyscallStatusV1::Ok as u64
        );
        assert_eq!(frame.registers[Arm64SyscallFrame::VALUE0_REGISTER], 0xAA);
        assert_eq!(frame.registers[Arm64SyscallFrame::VALUE1_REGISTER], 0xBB);
        assert_eq!(frame.elr, 0x4004);
        assert!(
            console
                .as_str()
                .contains("syscall trace request from initd")
        );
    }

    #[test]
    fn handle_syscall_exception_rejects_non_svc_sync() {
        let request = NovaSyscallRequestV1::new(NovaSyscallNumberV1::Nop, 0, [0; 6]);
        let mut frame = Arm64SyscallFrame::from_request(request);
        let original = frame;
        let mut console = RecordingConsole::new();

        let handled = handle_syscall_exception(
            (ExceptionClass::DataAbortLowerEl as u32) << 26,
            &mut frame,
            &SyscallDispatchState::scaffold(),
            &mut console,
        );

        assert!(!handled);
        assert_eq!(frame, original);
    }

    #[test]
    fn endpoint_call_requires_capabilities_and_endpoints() {
        let mut console = RecordingConsole::new();
        let result = dispatch_syscall(
            &SyscallDispatchState::scaffold(),
            NovaSyscallRequestV1::new(NovaSyscallNumberV1::EndpointCall, 0, [0; 6]),
            &mut console,
        );

        assert_eq!(result.status, NovaSyscallStatusV1::Denied as u32);
    }

    #[test]
    fn yield_requires_bootstrap_capability() {
        let mut console = RecordingConsole::new();
        let denied = dispatch_syscall(
            &SyscallDispatchState::scaffold(),
            NovaSyscallRequestV1::new(NovaSyscallNumberV1::Yield, 0, [0; 6]),
            &mut console,
        );
        let unsupported = dispatch_syscall(
            &SyscallDispatchState::bootstrap(
                initd_task(NovaInitCapsuleCapabilityV1::Yield as u64, 0, 0),
                false,
                false,
            ),
            NovaSyscallRequestV1::new(NovaSyscallNumberV1::Yield, 0, [0; 6]),
            &mut console,
        );

        assert_eq!(denied.status, NovaSyscallStatusV1::Denied as u32);
        assert_eq!(unsupported.status, NovaSyscallStatusV1::Unsupported as u32);
    }

    #[test]
    fn endpoint_call_requires_ready_endpoint_lane_even_with_capability() {
        let mut console = RecordingConsole::new();
        let result = dispatch_syscall(
            &SyscallDispatchState::bootstrap(
                initd_task(NovaInitCapsuleCapabilityV1::EndpointBootstrap as u64, 1, 0),
                false,
                false,
            ),
            NovaSyscallRequestV1::new(NovaSyscallNumberV1::EndpointCall, 0, [0; 6]),
            &mut console,
        );

        assert_eq!(result.status, NovaSyscallStatusV1::Unsupported as u32);
    }

    #[test]
    fn endpoint_call_round_trips_reserved_slot_when_ready() {
        let mut console = RecordingConsole::new();
        let result = dispatch_syscall(
            &SyscallDispatchState::bootstrap(
                initd_task(NovaInitCapsuleCapabilityV1::EndpointBootstrap as u64, 1, 0),
                true,
                false,
            ),
            NovaSyscallRequestV1::new(
                NovaSyscallNumberV1::EndpointCall,
                0,
                [0, 0x454E_4450, 0, 0, 0, 0],
            ),
            &mut console,
        );

        assert_eq!(result.status, NovaSyscallStatusV1::Ok as u32);
        assert_eq!(result.value0, 0);
        assert_eq!(result.value1, 0x454E_4450);
        assert!(
            console
                .as_str()
                .contains("bootstrap endpoint call from initd")
        );
    }

    #[test]
    fn shared_memory_map_requires_ready_lane_even_with_capability() {
        let mut console = RecordingConsole::new();
        let result = dispatch_syscall(
            &SyscallDispatchState::bootstrap(
                initd_task(
                    NovaInitCapsuleCapabilityV1::SharedMemoryBootstrap as u64,
                    0,
                    1,
                ),
                false,
                false,
            ),
            NovaSyscallRequestV1::new(NovaSyscallNumberV1::SharedMemoryMap, 0, [0; 6]),
            &mut console,
        );

        assert_eq!(result.status, NovaSyscallStatusV1::Unsupported as u32);
    }

    #[test]
    fn shared_memory_map_round_trips_reserved_region_when_ready() {
        let mut console = RecordingConsole::new();
        let result = dispatch_syscall(
            &SyscallDispatchState::bootstrap(
                initd_task(
                    NovaInitCapsuleCapabilityV1::SharedMemoryBootstrap as u64,
                    0,
                    1,
                ),
                false,
                true,
            ),
            NovaSyscallRequestV1::new(
                NovaSyscallNumberV1::SharedMemoryMap,
                0,
                [0, 0x5348_4D45_4D30, 0, 0, 0, 0],
            ),
            &mut console,
        );

        assert_eq!(result.status, NovaSyscallStatusV1::Ok as u32);
        assert_eq!(result.value0, 0);
        assert_eq!(result.value1, 0x5348_4D45_4D30);
        assert!(
            console
                .as_str()
                .contains("bootstrap shared memory map from initd")
        );
    }

    #[test]
    fn endpoint_call_rejects_unreserved_slot() {
        let mut console = RecordingConsole::new();
        let result = dispatch_syscall(
            &SyscallDispatchState::bootstrap(
                initd_task(NovaInitCapsuleCapabilityV1::EndpointBootstrap as u64, 1, 0),
                false,
                false,
            ),
            NovaSyscallRequestV1::new(NovaSyscallNumberV1::EndpointCall, 0, [1, 0, 0, 0, 0, 0]),
            &mut console,
        );

        assert_eq!(result.status, NovaSyscallStatusV1::InvalidArgs as u32);
    }

    #[test]
    fn shared_memory_map_rejects_unreserved_region() {
        let mut console = RecordingConsole::new();
        let result = dispatch_syscall(
            &SyscallDispatchState::bootstrap(
                initd_task(
                    NovaInitCapsuleCapabilityV1::SharedMemoryBootstrap as u64,
                    0,
                    1,
                ),
                false,
                false,
            ),
            NovaSyscallRequestV1::new(NovaSyscallNumberV1::SharedMemoryMap, 0, [1, 0, 0, 0, 0, 0]),
            &mut console,
        );

        assert_eq!(result.status, NovaSyscallStatusV1::InvalidArgs as u32);
    }

    #[test]
    fn dispatch_state_reports_current_task_identity() {
        let state = SyscallDispatchState::bootstrap(
            initd_task(NovaInitCapsuleCapabilityV1::BootLog as u64, 0, 0),
            false,
            false,
        );

        assert_eq!(state.current_task_service_name(), Some("initd"));
    }

    #[test]
    fn bootstrap_exception_handler_uses_installed_task_state() {
        let request =
            NovaSyscallRequestV1::new(NovaSyscallNumberV1::Trace, 0, [0xAA, 0xBB, 0, 0, 0, 0]);
        let mut frame = Arm64SyscallFrame::from_request(request);
        let mut console = RecordingConsole::new();
        frame.elr = 0x4000;
        install_bootstrap_syscall_state(SyscallDispatchState::bootstrap(
            initd_task(NovaInitCapsuleCapabilityV1::BootLog as u64, 0, 0),
            false,
            false,
        ));

        let handled = handle_bootstrap_syscall_exception(
            ((ExceptionClass::Brk64 as u32) << 26) | NOVA_BOOTSTRAP_TRAP_IMM16 as u32,
            &mut frame,
            &mut console,
        );

        assert!(handled);
        assert_eq!(
            bootstrap_syscall_state().current_task_service_name(),
            Some("initd")
        );
        assert_eq!(
            frame.registers[Arm64SyscallFrame::STATUS_REGISTER],
            NovaSyscallStatusV1::Ok as u64
        );
        assert_eq!(frame.elr, 0x4004);
        assert!(console.as_str().contains("bootstrap live trap from initd"));
        assert!(
            console
                .as_str()
                .contains("syscall trace request from initd")
        );
    }

    #[test]
    fn bootstrap_svc_exception_keeps_current_el_return_address() {
        let request =
            NovaSyscallRequestV1::new(NovaSyscallNumberV1::Trace, 0, [0xAA, 0xBB, 0, 0, 0, 0]);
        let mut frame = Arm64SyscallFrame::from_request(request);
        let mut console = RecordingConsole::new();
        frame.elr = 0x4004;
        install_bootstrap_syscall_state(SyscallDispatchState::bootstrap(
            initd_task(NovaInitCapsuleCapabilityV1::BootLog as u64, 0, 0),
            false,
            false,
        ));

        let handled = handle_bootstrap_syscall_exception(
            (ExceptionClass::Svc64 as u32) << 26,
            &mut frame,
            &mut console,
        );

        assert!(handled);
        assert_eq!(
            frame.registers[Arm64SyscallFrame::STATUS_REGISTER],
            NovaSyscallStatusV1::Ok as u64
        );
        assert_eq!(frame.registers[Arm64SyscallFrame::VALUE0_REGISTER], 0xAA);
        assert_eq!(frame.registers[Arm64SyscallFrame::VALUE1_REGISTER], 0xBB);
        assert_eq!(frame.elr, 0x4004);
        assert!(console.as_str().contains("bootstrap live svc from initd"));
    }

    #[test]
    fn bootstrap_lower_el_svc_exception_advances_return_address() {
        let request =
            NovaSyscallRequestV1::new(NovaSyscallNumberV1::Trace, 0, [0xAA, 0xBB, 0, 0, 0, 0]);
        let mut frame = Arm64SyscallFrame::from_request(request);
        let mut console = RecordingConsole::new();
        frame.elr = 0x4000;
        install_bootstrap_syscall_state(SyscallDispatchState::bootstrap(
            initd_task(NovaInitCapsuleCapabilityV1::BootLog as u64, 0, 0),
            false,
            false,
        ));

        let handled = handle_lower_el_bootstrap_syscall_exception(
            (ExceptionClass::Svc64 as u32) << 26,
            &mut frame,
            &mut console,
        );

        assert!(handled);
        assert_eq!(
            frame.registers[Arm64SyscallFrame::STATUS_REGISTER],
            NovaSyscallStatusV1::Ok as u64
        );
        assert_eq!(frame.registers[Arm64SyscallFrame::VALUE0_REGISTER], 0xAA);
        assert_eq!(frame.registers[Arm64SyscallFrame::VALUE1_REGISTER], 0xBB);
        assert_eq!(frame.elr, 0x4004);
        assert!(
            console
                .as_str()
                .contains("bootstrap lower-el svc from initd")
        );
    }

    #[test]
    fn bootstrap_lower_el_rejects_non_svc_exception() {
        let mut frame = Arm64SyscallFrame::empty();
        let mut console = RecordingConsole::new();
        frame.elr = 0x4000;

        let handled = handle_lower_el_bootstrap_syscall_exception(
            ((ExceptionClass::Brk64 as u32) << 26) | NOVA_BOOTSTRAP_TRAP_IMM16 as u32,
            &mut frame,
            &mut console,
        );

        assert!(!handled);
        assert_eq!(frame.elr, 0x4000);
        assert_eq!(console.as_str(), "");
    }

    struct RecordingConsole {
        bytes: [u8; 128],
        len: usize,
    }

    impl RecordingConsole {
        const fn new() -> Self {
            Self {
                bytes: [0; 128],
                len: 0,
            }
        }

        fn as_str(&self) -> &str {
            core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
        }
    }

    fn initd_task(
        capabilities: u64,
        endpoint_slots: u32,
        shared_memory_regions: u32,
    ) -> CurrentTaskState {
        CurrentTaskState::new(
            [
                b'i', b'n', b'i', b't', b'd', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
            BootstrapTaskState::new(capabilities, endpoint_slots, shared_memory_regions),
        )
    }

    impl ConsoleSink for RecordingConsole {
        fn write_str(&mut self, s: &str) {
            for &byte in s.as_bytes() {
                if self.len == self.bytes.len() {
                    break;
                }
                self.bytes[self.len] = byte;
                self.len += 1;
            }
        }
    }
}
