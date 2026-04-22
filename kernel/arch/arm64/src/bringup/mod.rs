use crate::boot_contract::{BootstrapCapsuleSummary, KernelBringupState, prepare_bringup};
use crate::bootinfo::{NovaBootInfoV1, NovaBootInfoV2};
#[cfg(not(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_kernel_svc_probe"
)))]
use crate::bootstrap;
use crate::console::{self, BootConsole, ConsoleSink};
#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "bootstrap_kernel_svc_probe"
))]
use crate::diag::run_bootstrap_kernel_svc_probe;
use crate::diag::run_syscall_probe;
use crate::exception_runtime::{
    bootstrap_syscall_dispatch_state, install_bootstrap_exception_runtime,
};
use crate::panic;

pub struct KernelContext<'a, C: ConsoleSink> {
    pub boot_info: &'a NovaBootInfoV1,
    pub boot_info_v2: Option<&'a NovaBootInfoV2>,
    pub bringup: Option<KernelBringupState>,
    pub console: &'a mut C,
}

pub fn kernel_main<C: ConsoleSink>(context: KernelContext<'_, C>) -> ! {
    context
        .console
        .log(console::LogLevel::Info, "NovaOS kernel bring-up");

    if !context.boot_info.is_valid() {
        context
            .console
            .log(console::LogLevel::Warn, "boot info marker is not set");
    }

    let summary = context
        .bringup
        .map(|bringup| bringup.boot_summary)
        .unwrap_or_else(|| context.boot_info.summary());
    context
        .console
        .log(console::LogLevel::Info, summary.describe());

    if context.boot_info_v2.is_some() {
        context
            .console
            .log(console::LogLevel::Info, "boot info v2 summary observed");
    }

    let bringup = context.bringup.unwrap_or_else(|| {
        prepare_bringup(context.boot_info, context.boot_info_v2)
            .unwrap_or_else(KernelBringupState::empty)
    });
    let vectors = bringup.exception_vectors;
    let _page_tables = bringup.page_tables;
    let _allocator = bringup.allocator;

    if let Some(init_capsule) = bringup.init_capsule {
        log_init_capsule_summary(context.console, init_capsule);
    }

    let bootstrap_syscall_state = bootstrap_syscall_dispatch_state(bringup.init_capsule);
    run_syscall_probe(context.console, bootstrap_syscall_state);
    install_bootstrap_exception_runtime(vectors, bootstrap_syscall_state);
    #[cfg(all(
        target_os = "none",
        target_arch = "aarch64",
        feature = "bootstrap_kernel_svc_probe"
    ))]
    run_bootstrap_kernel_svc_probe();

    #[cfg(not(all(
        target_os = "none",
        target_arch = "aarch64",
        feature = "bootstrap_kernel_svc_probe"
    )))]
    {
        let bootstrap_launch_plan = bringup
            .init_capsule
            .and_then(|init_capsule| init_capsule.launch_plan());
        if let Some(launch_plan) = bootstrap_launch_plan {
            context.console.write_str("[info] bootstrap task transfer ");
            context.console.write_line(launch_plan.service_name());
            bootstrap::enter_bootstrap_task(
                context.console,
                launch_plan,
                bringup.init_capsule,
                bringup.page_tables,
                bringup.allocator,
            );
        }
    }

    panic::log_and_halt(context.console, "kernel bring-up remains a scaffold");
}

pub(crate) fn enter_kernel_runtime(
    boot_info: &'static NovaBootInfoV1,
    boot_info_v2: Option<&'static NovaBootInfoV2>,
    bringup: Option<KernelBringupState>,
) -> ! {
    let mut console = BootConsole::from_boot_info(boot_info);
    kernel_main(KernelContext {
        boot_info,
        boot_info_v2,
        bringup,
        console: &mut console,
    })
}

pub(crate) fn log_init_capsule_summary<C: ConsoleSink>(
    console: &mut C,
    init_capsule: BootstrapCapsuleSummary,
) {
    console.log(console::LogLevel::Info, "init capsule summary observed");
    console.write_str("[info] init capsule service ");
    console.write_line(init_capsule.service_name());
    console.write_str("[info] bootstrap task current ");
    console.write_line(init_capsule.service_name());
    if init_capsule.payload_body_present {
        console.log(console::LogLevel::Info, "bootstrap task image observed");
    }
    if let Some(launch_plan) = init_capsule.launch_plan() {
        let _ = launch_plan;
        if init_capsule.payload_descriptor_from_boot_info_v2 {
            console.log(
                console::LogLevel::Info,
                "bootstrap task launch plan from bootinfo_v2",
            );
        } else if init_capsule.payload_body_present {
            console.log(console::LogLevel::Info, "bootstrap task image staged");
        }
    } else if init_capsule.payload_body_present {
        console.log(console::LogLevel::Info, "bootstrap task image staged");
    }
}
