#![no_std]

pub mod runtime;

pub use runtime::{
    ACCELD_DESCRIPTOR, AGENTD_DESCRIPTOR, APPBRIDGED_DESCRIPTOR, CORE_SERVICE_BOOT_STATUSES,
    CORE_SERVICE_KERNEL_LAUNCH_PLANS, CORE_SERVICE_LAUNCH_ORDER, CORE_SERVICE_LAUNCH_SPECS,
    INTENTD_DESCRIPTOR, InitKernelLaunchPlanPage, InitRuntimeSnapshot, InitRuntimeStatusPage,
    InitServiceLaunchPlan, InitServiceLaunchTable, MEMD_DESCRIPTOR, POLICYD_DESCRIPTOR,
    SCENED_DESCRIPTOR, SHELLD_DESCRIPTOR, core_launch_plan, core_launch_table, initd_boot_snapshot,
    initd_boot_status_page, initd_descriptor, initd_kernel_launch_plan_page,
};

#[cfg(test)]
mod tests;
