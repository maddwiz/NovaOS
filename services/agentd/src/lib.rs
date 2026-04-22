#![no_std]

pub mod launch;
pub mod types;

pub use launch::{AGENTD_DESCRIPTOR, AGENTD_LAUNCH_SPEC};
pub use types::{
    AgentCapabilityBundle, AgentControlEvent, AgentDescriptor, AgentLifecycleState,
    AgentStateMachine,
};

#[cfg(test)]
mod tests;
