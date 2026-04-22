#![no_std]

pub mod runtime;

pub use runtime::{
    ACCELD_DESCRIPTOR, ACCELD_LAUNCH_SPEC, AGENTD_DESCRIPTOR, AGENTD_LAUNCH_SPEC,
    APPBRIDGED_DESCRIPTOR, APPBRIDGED_LAUNCH_SPEC, CORE_SERVICE_BOOT_STATUSES,
    CORE_SERVICE_KERNEL_LAUNCH_PLANS, CORE_SERVICE_LAUNCH_ORDER, CORE_SERVICE_LAUNCH_SPECS,
    INTENTD_DESCRIPTOR, INTENTD_LAUNCH_SPEC, InitKernelLaunchPlanPage, InitRuntimeReport,
    InitRuntimeServiceReport, InitRuntimeSnapshot, InitRuntimeStatusPage, InitServiceLaunchPlan,
    InitServiceLaunchTable, MEMD_DESCRIPTOR, MEMD_LAUNCH_SPEC, POLICYD_DESCRIPTOR,
    POLICYD_LAUNCH_SPEC, SCENED_DESCRIPTOR, SCENED_LAUNCH_SPEC, SHELLD_DESCRIPTOR,
    SHELLD_LAUNCH_SPEC, core_launch_plan, core_launch_table, initd_boot_snapshot,
    initd_boot_status_page, initd_descriptor, initd_kernel_launch_plan_page, initd_runtime_report,
};

#[cfg(test)]
mod tests;
