use crate::{
    INTENTD_LAUNCH_SPEC, policy_request_for_intent, route_intent, route_intent_with_policy,
};
use nova_rt::{
    NovaAgentId, NovaIntentEnvelope, NovaIntentKind, NovaPolicyAction, NovaPolicyDecision,
    NovaPolicyScope, NovaSceneId, NovaServiceId,
};

#[test]
fn open_app_intent_routes_to_app_bridge() {
    let plan = route_intent(NovaIntentEnvelope {
        id: 1,
        source_agent: NovaAgentId::INIT,
        scene: NovaSceneId::ROOT,
        target_service: NovaServiceId::new(0),
        kind: NovaIntentKind::OpenApp,
        policy_hint: NovaPolicyDecision::Ask,
    });

    assert_eq!(plan.primary_service, NovaServiceId::APPBRIDGED);
    assert!(plan.requires_approval);
}

#[test]
fn status_intent_routes_to_shell() {
    let plan = route_intent(NovaIntentEnvelope {
        id: 2,
        source_agent: NovaAgentId::INIT,
        scene: NovaSceneId::ROOT,
        target_service: NovaServiceId::new(0),
        kind: NovaIntentKind::RequestStatus,
        policy_hint: NovaPolicyDecision::Allow,
    });

    assert_eq!(plan.primary_service, NovaServiceId::SHELLD);
    assert!(!plan.requires_approval);
}

#[test]
fn switch_scene_intent_routes_to_scene_service() {
    let plan = route_intent(NovaIntentEnvelope {
        id: 3,
        source_agent: NovaAgentId::INIT,
        scene: NovaSceneId::ROOT,
        target_service: NovaServiceId::new(0),
        kind: NovaIntentKind::SwitchScene,
        policy_hint: NovaPolicyDecision::Ask,
    });

    assert_eq!(plan.primary_service, NovaServiceId::SCENED);
    assert_eq!(plan.step.target_service, NovaServiceId::SCENED);
    assert!(plan.requires_approval);
}

#[test]
fn explicit_target_overrides_default_route() {
    let plan = route_intent(NovaIntentEnvelope {
        id: 4,
        source_agent: NovaAgentId::INIT,
        scene: NovaSceneId::ROOT,
        target_service: NovaServiceId::AGENTD,
        kind: NovaIntentKind::OpenApp,
        policy_hint: NovaPolicyDecision::Ask,
    });

    assert_eq!(plan.primary_service, NovaServiceId::AGENTD);
    assert_eq!(plan.step.target_service, NovaServiceId::AGENTD);
}

#[test]
fn intent_policy_projection_routes_through_scene_scope() {
    let intent = NovaIntentEnvelope {
        id: 5,
        source_agent: NovaAgentId::new(77),
        scene: NovaSceneId::ROOT,
        target_service: NovaServiceId::new(0),
        kind: NovaIntentKind::SwitchScene,
        policy_hint: NovaPolicyDecision::Deny,
    };
    let projection = policy_request_for_intent(intent);

    assert_eq!(projection.intent_id, 5);
    assert_eq!(projection.request.subject_service, NovaServiceId::INTENTD);
    assert_eq!(projection.request.subject_agent, NovaAgentId::new(77));
    assert_eq!(projection.request.action, NovaPolicyAction::RouteIntent);
    assert_eq!(
        projection.request.scope,
        NovaPolicyScope::Scene(NovaSceneId::ROOT)
    );
}

#[test]
fn route_with_policy_decision_replaces_hint() {
    let intent = NovaIntentEnvelope {
        id: 6,
        source_agent: NovaAgentId::INIT,
        scene: NovaSceneId::ROOT,
        target_service: NovaServiceId::new(0),
        kind: NovaIntentKind::RequestStatus,
        policy_hint: NovaPolicyDecision::Deny,
    };
    let plan = route_intent_with_policy(intent, NovaPolicyDecision::Ask);

    assert_eq!(plan.primary_service, NovaServiceId::SHELLD);
    assert_eq!(plan.step.policy, NovaPolicyDecision::Ask);
    assert!(plan.requires_approval);
}

#[test]
fn launch_spec_identifies_intent_service() {
    assert_eq!(INTENTD_LAUNCH_SPEC.descriptor.id, NovaServiceId::INTENTD);
    assert!(INTENTD_LAUNCH_SPEC.is_valid());
}
