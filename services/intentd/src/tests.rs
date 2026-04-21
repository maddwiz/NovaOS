use crate::route_intent;
use nova_rt::{
    NovaAgentId, NovaIntentEnvelope, NovaIntentKind, NovaPolicyDecision, NovaSceneId, NovaServiceId,
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
