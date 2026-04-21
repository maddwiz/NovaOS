#![no_std]

pub mod runtime;

pub use runtime::{
    CORE_SERVICE_LAUNCH_ORDER, InitRuntimeSnapshot, InitServiceLaunchTable, core_launch_table,
    initd_boot_snapshot, initd_descriptor,
};

#[cfg(test)]
mod tests;
