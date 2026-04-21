#![no_std]

pub mod types;

pub use types::{PolicyMatrix, PolicyRule, default_policy_matrix, evaluate_policy};

#[cfg(test)]
mod tests;
