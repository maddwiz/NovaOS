use nova_rt::{
    NovaIntentEnvelope, NovaIntentKind, NovaPolicyAction, NovaPolicyDecision, NovaPolicyRequest,
    NovaPolicyScope, NovaServiceId, NovaServiceLaunchStatus,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IntentPolicyProjection {
    pub intent_id: u64,
    pub request: NovaPolicyRequest,
}

pub const fn policy_request_for_intent(intent: NovaIntentEnvelope) -> IntentPolicyProjection {
    IntentPolicyProjection {
        intent_id: intent.id,
        request: NovaPolicyRequest {
            subject_service: NovaServiceId::INTENTD,
            subject_agent: intent.source_agent,
            action: NovaPolicyAction::RouteIntent,
            scope: NovaPolicyScope::Scene(intent.scene),
        },
    }
}

pub const fn route_intent_with_policy(
    intent: NovaIntentEnvelope,
    decision: NovaPolicyDecision,
) -> IntentPlan {
    route_intent(NovaIntentEnvelope {
        policy_hint: decision,
        ..intent
    })
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
