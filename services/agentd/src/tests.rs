use crate::{
    AGENTD_LAUNCH_SPEC, AgentCapabilityBundle, AgentControlEvent, AgentDescriptor,
    AgentLifecycleState, AgentQuotaStatus, AgentRuntimeRecord, AgentSceneParticipationStatus,
    AgentStateMachine,
};
use nova_rt::{NovaAgentId, NovaSceneId, NovaServiceId};

fn planner_descriptor() -> AgentDescriptor {
    AgentDescriptor {
        id: NovaAgentId::new(42),
        name: "planner",
        owner_service: NovaServiceId::AGENTD,
        capabilities: AgentCapabilityBundle {
            tool_grants: 2,
            service_grants: 3,
            memory_budget_pages: 128,
        },
    }
}

#[test]
fn agent_lifecycle_reaches_running_after_ready() {
    let descriptor = planner_descriptor();

    let machine = AgentStateMachine::new(descriptor)
        .apply(AgentControlEvent::Launch)
        .apply(AgentControlEvent::Ready);

    assert_eq!(machine.state, AgentLifecycleState::Running);
    assert!(machine.is_running());
    assert_eq!(machine.descriptor.capabilities.tool_grants, 2);
}

#[test]
fn agent_labels_are_stable_for_operator_reports() {
    assert_eq!(AgentLifecycleState::Declared.label(), "declared");
    assert_eq!(AgentLifecycleState::Running.label(), "running");
    assert_eq!(AgentControlEvent::Launch.label(), "launch");
    assert_eq!(AgentControlEvent::Fail.label(), "fail");
    assert_eq!(AgentQuotaStatus::Allowed.label(), "allowed");
    assert_eq!(
        AgentSceneParticipationStatus::SceneMismatch.label(),
        "scene-mismatch"
    );
}

#[test]
fn running_agent_runtime_allows_quota_within_bundle() {
    let runtime = AgentRuntimeRecord::from_descriptor(planner_descriptor(), NovaSceneId::ROOT)
        .apply(AgentControlEvent::Launch)
        .apply(AgentControlEvent::Ready)
        .with_usage(1, 2, 64);

    let snapshot = runtime.quota_snapshot();
    assert_eq!(snapshot.remaining_tool_grants(), 1);
    assert_eq!(snapshot.remaining_service_grants(), 1);
    assert_eq!(snapshot.remaining_memory_pages(), 64);

    assert!(runtime.check_tool_grants(1).allowed());
    assert!(runtime.check_service_delegation(1).allowed());
    assert!(runtime.check_memory_pages(64).allowed());
}

#[test]
fn agent_runtime_rejects_quota_overruns() {
    let runtime = AgentRuntimeRecord::from_descriptor(planner_descriptor(), NovaSceneId::ROOT)
        .apply(AgentControlEvent::Launch)
        .apply(AgentControlEvent::Ready)
        .with_usage(1, 2, 64);

    let tool_decision = runtime.check_tool_grants(2);
    assert_eq!(tool_decision.status, AgentQuotaStatus::ToolGrantExceeded);
    assert_eq!(tool_decision.used, 1);
    assert_eq!(tool_decision.limit, 2);

    let service_decision = runtime.check_service_delegation(2);
    assert_eq!(
        service_decision.status,
        AgentQuotaStatus::ServiceGrantExceeded
    );

    let memory_decision = runtime.check_memory_pages(65);
    assert_eq!(
        memory_decision.status,
        AgentQuotaStatus::MemoryBudgetExceeded
    );
}

#[test]
fn agent_runtime_rejects_quota_when_agent_not_running() {
    let runtime = AgentRuntimeRecord::from_descriptor(planner_descriptor(), NovaSceneId::ROOT);

    assert_eq!(
        runtime.check_tool_grants(1).status,
        AgentQuotaStatus::NotRunning
    );
    assert_eq!(
        runtime.check_service_delegation(1).status,
        AgentQuotaStatus::NotRunning
    );
    assert_eq!(
        runtime.check_memory_pages(1).status,
        AgentQuotaStatus::NotRunning
    );
}

#[test]
fn scene_participation_requires_running_agent_attached_to_scene() {
    let runtime = AgentRuntimeRecord::from_descriptor(planner_descriptor(), NovaSceneId::ROOT)
        .apply(AgentControlEvent::Launch)
        .apply(AgentControlEvent::Ready);

    assert!(runtime.scene_participation(NovaSceneId::ROOT).allowed());
    assert_eq!(
        runtime.scene_participation(NovaSceneId::new(2)).status,
        AgentSceneParticipationStatus::SceneMismatch
    );

    let stopped = runtime.apply(AgentControlEvent::Stop);
    assert_eq!(
        stopped.scene_participation(NovaSceneId::ROOT).status,
        AgentSceneParticipationStatus::NotRunning
    );
}

#[test]
fn launch_spec_identifies_agent_service() {
    assert_eq!(AGENTD_LAUNCH_SPEC.descriptor.id, NovaServiceId::AGENTD);
    assert!(AGENTD_LAUNCH_SPEC.is_valid());
}
