#![no_std]

pub mod launch;
pub mod types;

pub use launch::{INTENTD_DESCRIPTOR, INTENTD_LAUNCH_SPEC};
pub use types::{IntentPlan, IntentPlanStep, route_intent};

#[cfg(test)]
mod tests;
