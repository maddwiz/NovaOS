#![no_std]

pub mod launch;
pub mod types;

pub use launch::{POLICYD_DESCRIPTOR, POLICYD_LAUNCH_SPEC};
pub use types::{PolicyMatrix, PolicyRule, default_policy_matrix, evaluate_policy};

#[cfg(test)]
mod tests;
