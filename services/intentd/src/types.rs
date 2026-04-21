use nova_rt::{
    NovaIntentEnvelope, NovaIntentKind, NovaPolicyDecision, NovaServiceId, NovaServiceLaunchStatus,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IntentPlanStep {
    pub target_service: NovaServiceId,
    pub policy: NovaPolicyDecision,
    pub launch_status: NovaServiceLaunchStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IntentPlan {
    pub intent_id: u64,
    pub primary_service: NovaServiceId,
    pub step: IntentPlanStep,
    pub requires_approval: bool,
}

pub const fn route_intent(intent: NovaIntentEnvelope) -> IntentPlan {
    let primary_service = if !intent.target_service.is_empty() {
        intent.target_service
    } else {
        match intent.kind {
            NovaIntentKind::LaunchService | NovaIntentKind::RequestStatus => NovaServiceId::SHELLD,
            NovaIntentKind::OpenApp => NovaServiceId::APPBRIDGED,
            NovaIntentKind::SwitchScene => NovaServiceId::SCENED,
            NovaIntentKind::Custom => NovaServiceId::AGENTD,
        }
    };
    let requires_approval = matches!(intent.policy_hint, NovaPolicyDecision::Ask);

    IntentPlan {
        intent_id: intent.id,
        primary_service,
        step: IntentPlanStep {
            target_service: primary_service,
            policy: intent.policy_hint,
            launch_status: NovaServiceLaunchStatus::Deferred,
        },
        requires_approval,
    }
}
