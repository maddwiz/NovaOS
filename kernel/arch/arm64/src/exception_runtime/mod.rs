use crate::arch::arm64::exceptions::ExceptionVectors;
use crate::boot_contract::BootstrapCapsuleSummary;
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_trap_vector_trace"
))]
use crate::diag::log_bootstrap_exception_install_status;
use crate::syscall::{CurrentTaskState, SyscallDispatchState, install_bootstrap_syscall_state};
use nova_rt::NovaInitCapsuleCapabilityV1;

pub(crate) fn install_bootstrap_exception_runtime(
    vectors: ExceptionVectors,
    bootstrap_syscall_state: SyscallDispatchState,
) {
    install_bootstrap_syscall_state(bootstrap_syscall_state);
    let _installed_vectors = unsafe { vectors.install() };
    #[cfg(all(
        target_os = "none",
        target_arch = "aarch64",
        feature = "bootstrap_trap_vector_trace"
    ))]
    log_bootstrap_exception_install_status(vectors, _installed_vectors);
}

pub(crate) fn bootstrap_syscall_dispatch_state(
    init_capsule: Option<BootstrapCapsuleSummary>,
) -> SyscallDispatchState {
    init_capsule
        .map(|init_capsule| {
            let task_state = init_capsule.task_state();
            let endpoints_ready = task_state
                .has_bootstrap_capability(NovaInitCapsuleCapabilityV1::EndpointBootstrap)
                && task_state.endpoint_slots != 0;
            let shared_memory_ready = task_state
                .has_bootstrap_capability(NovaInitCapsuleCapabilityV1::SharedMemoryBootstrap)
                && task_state.shared_memory_regions != 0;
            SyscallDispatchState::bootstrap(
                CurrentTaskState::new(init_capsule.service_name, task_state),
                endpoints_ready,
                shared_memory_ready,
            )
        })
        .unwrap_or_else(SyscallDispatchState::scaffold)
}
