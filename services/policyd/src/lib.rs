#![no_std]

pub mod launch;
pub mod types;

pub use launch::{POLICYD_DESCRIPTOR, POLICYD_LAUNCH_SPEC, POLICYD_PAYLOAD_SPEC};
pub use types::{
    POLICY_AUDIT_NO_RULE, PolicyAuditRecord, PolicyDecisionSource, PolicyMatrix, PolicyRule,
    default_policy_matrix, evaluate_policy, evaluate_policy_with_audit,
};

#[cfg(test)]
mod tests;
