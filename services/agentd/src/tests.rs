use crate::{
    AGENTD_LAUNCH_SPEC, AgentCapabilityBundle, AgentControlEvent, AgentDescriptor,
    AgentLifecycleState, AgentStateMachine,
};
use nova_rt::{NovaAgentId, NovaServiceId};

#[test]
fn agent_lifecycle_reaches_running_after_ready() {
    let descriptor = AgentDescriptor {
        id: NovaAgentId::new(42),
        name: "planner",
        owner_service: NovaServiceId::AGENTD,
        capabilities: AgentCapabilityBundle {
            tool_grants: 2,
            service_grants: 3,
            memory_budget_pages: 128,
        },
    };

    let machine = AgentStateMachine::new(descriptor)
        .apply(AgentControlEvent::Launch)
        .apply(AgentControlEvent::Ready);

    assert_eq!(machine.state, AgentLifecycleState::Running);
    assert_eq!(machine.descriptor.capabilities.tool_grants, 2);
}

#[test]
fn launch_spec_identifies_agent_service() {
    assert_eq!(AGENTD_LAUNCH_SPEC.descriptor.id, NovaServiceId::AGENTD);
    assert!(AGENTD_LAUNCH_SPEC.is_valid());
}
