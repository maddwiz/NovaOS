use nova_rt::{
    NovaAppActionKind, NovaAppActionRequest, NovaIntentDispatch, NovaIntentEnvelope,
    NovaIntentKind, NovaIntentProjection, NovaPolicyAction, NovaPolicyDecision, NovaPolicyRequest,
    NovaPolicyScope, NovaSceneSwitchRequest, NovaServiceId, NovaServiceLaunchRequest,
    NovaServiceLaunchStatus, NovaStatusRequest,
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
    pub projection: NovaIntentProjection,
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
    let primary_service = resolve_primary_service(intent);
    let requires_approval = matches!(intent.policy_hint, NovaPolicyDecision::Ask);

    IntentPlan {
        intent_id: intent.id,
        primary_service,
        step: IntentPlanStep {
            target_service: primary_service,
            policy: intent.policy_hint,
            launch_status: NovaServiceLaunchStatus::Deferred,
        },
        projection: project_intent(intent, primary_service),
        requires_approval,
    }
}

const fn resolve_primary_service(intent: NovaIntentEnvelope) -> NovaServiceId {
    if !intent.target_service.is_empty() && !matches!(intent.kind, NovaIntentKind::LaunchService) {
        intent.target_service
    } else {
        match intent.kind {
            NovaIntentKind::LaunchService | NovaIntentKind::RequestStatus => NovaServiceId::SHELLD,
            NovaIntentKind::OpenApp => NovaServiceId::APPBRIDGED,
            NovaIntentKind::SwitchScene => NovaServiceId::SCENED,
            NovaIntentKind::Custom => NovaServiceId::AGENTD,
        }
    }
}

const fn project_intent(
    intent: NovaIntentEnvelope,
    primary_service: NovaServiceId,
) -> NovaIntentProjection {
    let dispatch = match intent.kind {
        NovaIntentKind::LaunchService => {
            NovaIntentDispatch::LaunchService(NovaServiceLaunchRequest::new(
                NovaServiceId::INTENTD,
                resolve_launch_target(intent, primary_service),
                intent.scene,
                0,
            ))
        }
        NovaIntentKind::OpenApp => NovaIntentDispatch::AppAction(NovaAppActionRequest::unresolved(
            intent.scene,
            intent.source_agent,
            NovaAppActionKind::Open,
        )),
        NovaIntentKind::SwitchScene => NovaIntentDispatch::SwitchScene(
            NovaSceneSwitchRequest::unresolved(intent.source_agent, intent.scene),
        ),
        NovaIntentKind::RequestStatus | NovaIntentKind::Custom => NovaIntentDispatch::Status(
            NovaStatusRequest::new(intent.source_agent, intent.scene, primary_service),
        ),
    };

    NovaIntentProjection::new(intent.id, primary_service, dispatch)
}

const fn resolve_launch_target(
    intent: NovaIntentEnvelope,
    primary_service: NovaServiceId,
) -> NovaServiceId {
    if !intent.target_service.is_empty() {
        intent.target_service
    } else {
        primary_service
    }
}
