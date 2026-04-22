use crate::{POLICYD_LAUNCH_SPEC, default_policy_matrix, evaluate_policy};
use nova_rt::{
    NovaAgentId, NovaPolicyAction, NovaPolicyDecision, NovaPolicyRequest, NovaPolicyScope,
    NovaSceneId, NovaServiceId,
};

#[test]
fn default_matrix_allows_core_service_launches() {
    let request = NovaPolicyRequest {
        subject_service: NovaServiceId::INITD,
        subject_agent: NovaAgentId::INIT,
        action: NovaPolicyAction::LaunchService,
        scope: NovaPolicyScope::Service(NovaServiceId::POLICYD),
    };

    assert_eq!(
        default_policy_matrix().decide(request),
        NovaPolicyDecision::Allow
    );
}

#[test]
fn default_matrix_denies_unknown_service_launches() {
    let request = NovaPolicyRequest {
        subject_service: NovaServiceId::INITD,
        subject_agent: NovaAgentId::INIT,
        action: NovaPolicyAction::LaunchService,
        scope: NovaPolicyScope::Service(NovaServiceId::new(0x9999)),
    };

    assert_eq!(evaluate_policy(request), NovaPolicyDecision::Deny);
}

#[test]
fn launch_spec_identifies_policy_service() {
    assert_eq!(POLICYD_LAUNCH_SPEC.descriptor.id, NovaServiceId::POLICYD);
    assert!(POLICYD_LAUNCH_SPEC.is_valid());
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
fn system_scope_rules_apply_as_global_defaults() {
    let route_intent = NovaPolicyRequest {
        subject_service: NovaServiceId::INTENTD,
        subject_agent: NovaAgentId::INIT,
        action: NovaPolicyAction::RouteIntent,
        scope: NovaPolicyScope::Scene(NovaSceneId::ROOT),
    };
    let app_action = NovaPolicyRequest {
        subject_service: NovaServiceId::APPBRIDGED,
        subject_agent: NovaAgentId::INIT,
        action: NovaPolicyAction::AppAction,
        scope: NovaPolicyScope::Service(NovaServiceId::APPBRIDGED),
    };

    assert_eq!(evaluate_policy(route_intent), NovaPolicyDecision::Ask);
    assert_eq!(evaluate_policy(app_action), NovaPolicyDecision::Ask);
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
