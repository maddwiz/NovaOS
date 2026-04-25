#![no_std]

pub mod launch;
pub mod types;

pub use launch::{INTENTD_DESCRIPTOR, INTENTD_LAUNCH_SPEC, INTENTD_PAYLOAD_SPEC};
pub use types::{
    IntentPlan, IntentPlanStep, IntentPolicyProjection, policy_request_for_intent, route_intent,
    route_intent_with_policy,
};

#[cfg(test)]
mod tests;
