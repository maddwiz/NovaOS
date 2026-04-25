#![no_std]

pub mod launch;
pub mod types;

pub use launch::{AGENTD_DESCRIPTOR, AGENTD_LAUNCH_SPEC, AGENTD_PAYLOAD_SPEC};
pub use types::{
    AgentCapabilityBundle, AgentControlEvent, AgentDescriptor, AgentLifecycleState,
    AgentQuotaDecision, AgentQuotaSnapshot, AgentQuotaStatus, AgentRuntimeRecord,
    AgentSceneParticipation, AgentSceneParticipationStatus, AgentStateMachine,
};

#[cfg(test)]
mod tests;
