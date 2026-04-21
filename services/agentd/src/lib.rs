#![no_std]

pub mod types;

pub use types::{
    AgentCapabilityBundle, AgentControlEvent, AgentDescriptor, AgentLifecycleState,
    AgentStateMachine,
};

#[cfg(test)]
mod tests;
