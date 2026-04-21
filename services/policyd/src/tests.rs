use crate::{default_policy_matrix, evaluate_policy};
use nova_rt::{
    NovaAgentId, NovaPolicyAction, NovaPolicyDecision, NovaPolicyRequest, NovaPolicyScope,
    NovaServiceId,
};

#[test]
fn default_matrix_allows_core_service_launches() {
    let request = NovaPolicyRequest {
        subject_service: NovaServiceId::INITD,
        subject_agent: NovaAgentId::INIT,
        action: NovaPolicyAction::LaunchService,
        scope: NovaPolicyScope::System,
    };

    assert_eq!(
        default_policy_matrix().decide(request),
        NovaPolicyDecision::Allow
    );
}

#[test]
fn default_matrix_asks_before_agent_or_app_actions() {
    let request = NovaPolicyRequest {
        subject_service: NovaServiceId::AGENTD,
        subject_agent: NovaAgentId::new(7),
        action: NovaPolicyAction::AppAction,
        scope: NovaPolicyScope::System,
    };

    assert_eq!(evaluate_policy(request), NovaPolicyDecision::Ask);
}

#[test]
fn memory_visibility_defaults_to_deny() {
    let request = NovaPolicyRequest {
        subject_service: NovaServiceId::MEMD,
        subject_agent: NovaAgentId::INIT,
        action: NovaPolicyAction::AccessMemory,
        scope: NovaPolicyScope::System,
    };

    assert_eq!(evaluate_policy(request), NovaPolicyDecision::Deny);
}
