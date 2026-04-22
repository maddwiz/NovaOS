use crate::{
    POLICY_AUDIT_NO_RULE, POLICYD_LAUNCH_SPEC, PolicyDecisionSource, default_policy_matrix,
    evaluate_policy, evaluate_policy_with_audit,
};
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
fn audit_record_tracks_rule_backed_decision() {
    let request = NovaPolicyRequest {
        subject_service: NovaServiceId::INITD,
        subject_agent: NovaAgentId::INIT,
        action: NovaPolicyAction::LaunchService,
        scope: NovaPolicyScope::Service(NovaServiceId::POLICYD),
    };

    let audit = evaluate_policy_with_audit(request, 7);

    assert_eq!(audit.sequence, 7);
    assert_eq!(audit.decision, NovaPolicyDecision::Allow);
    assert_eq!(audit.source, PolicyDecisionSource::MatrixRule);
    assert_eq!(audit.matched_rule_index, 0);
    assert!(audit.matched_rule());
    assert!(audit.allowed());
}

#[test]
fn audit_record_tracks_default_denial() {
    let request = NovaPolicyRequest {
        subject_service: NovaServiceId::INITD,
        subject_agent: NovaAgentId::INIT,
        action: NovaPolicyAction::StopService,
        scope: NovaPolicyScope::Service(NovaServiceId::new(0x9999)),
    };

    let audit = evaluate_policy_with_audit(request, 9);

    assert_eq!(audit.decision, NovaPolicyDecision::Deny);
    assert_eq!(audit.source, PolicyDecisionSource::MatrixDefault);
    assert_eq!(audit.matched_rule_index, POLICY_AUDIT_NO_RULE);
    assert!(!audit.matched_rule());
    assert!(!audit.allowed());
    assert!(!audit.requires_approval());
}

#[test]
fn audit_record_tracks_self_policy_override() {
    let request = NovaPolicyRequest {
        subject_service: NovaServiceId::POLICYD,
        subject_agent: NovaAgentId::INIT,
        action: NovaPolicyAction::AccessMemory,
        scope: NovaPolicyScope::System,
    };

    let audit = evaluate_policy_with_audit(request, 11);

    assert_eq!(audit.decision, NovaPolicyDecision::Allow);
    assert_eq!(audit.source, PolicyDecisionSource::PolicydSelf);
    assert_eq!(audit.source.label(), "policyd-self");
    assert_eq!(audit.matched_rule_index, POLICY_AUDIT_NO_RULE);
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
    assert!(evaluate_policy_with_audit(request, 12).requires_approval());
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
