#![no_std]

pub mod runtime;

pub use runtime::{
    ACCELD_DESCRIPTOR, AGENTD_DESCRIPTOR, APPBRIDGED_DESCRIPTOR, CORE_SERVICE_BOOT_STATUSES,
    CORE_SERVICE_LAUNCH_ORDER, INTENTD_DESCRIPTOR, InitRuntimeSnapshot, InitRuntimeStatusPage,
    InitServiceLaunchTable, MEMD_DESCRIPTOR, POLICYD_DESCRIPTOR, SCENED_DESCRIPTOR,
    SHELLD_DESCRIPTOR, core_launch_table, initd_boot_snapshot, initd_boot_status_page,
    initd_descriptor,
};

#[cfg(test)]
mod tests;
