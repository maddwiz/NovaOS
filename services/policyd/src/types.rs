use nova_rt::{
    NovaPolicyAction, NovaPolicyDecision, NovaPolicyRequest, NovaPolicyScope, NovaServiceId,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PolicyRule {
    pub action: NovaPolicyAction,
    pub scope: NovaPolicyScope,
    pub decision: NovaPolicyDecision,
}

impl PolicyRule {
    pub const fn new(
        action: NovaPolicyAction,
        scope: NovaPolicyScope,
        decision: NovaPolicyDecision,
    ) -> Self {
        Self {
            action,
            scope,
            decision,
        }
    }

    pub const fn matches(self, request: NovaPolicyRequest) -> bool {
        self.action as u16 == request.action as u16
            && policy_scope_matches(self.scope, request.scope)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PolicyMatrix {
    pub rules: &'static [PolicyRule],
    pub default_decision: NovaPolicyDecision,
}

impl PolicyMatrix {
    pub const fn new(rules: &'static [PolicyRule], default_decision: NovaPolicyDecision) -> Self {
        Self {
            rules,
            default_decision,
        }
    }

    pub fn decide(self, request: NovaPolicyRequest) -> NovaPolicyDecision {
        for rule in self.rules {
            if rule.matches(request) {
                return rule.decision;
            }
        }
        self.default_decision
    }
}

const DEFAULT_POLICY_RULES: &[PolicyRule] = &[
    PolicyRule::new(
        NovaPolicyAction::LaunchService,
        NovaPolicyScope::System,
        NovaPolicyDecision::Allow,
    ),
    PolicyRule::new(
        NovaPolicyAction::RouteIntent,
        NovaPolicyScope::System,
        NovaPolicyDecision::Ask,
    ),
    PolicyRule::new(
        NovaPolicyAction::DelegateToAgent,
        NovaPolicyScope::System,
        NovaPolicyDecision::Ask,
    ),
    PolicyRule::new(
        NovaPolicyAction::AppAction,
        NovaPolicyScope::System,
        NovaPolicyDecision::Ask,
    ),
    PolicyRule::new(
        NovaPolicyAction::AccessMemory,
        NovaPolicyScope::System,
        NovaPolicyDecision::Deny,
    ),
];

pub const fn default_policy_matrix() -> PolicyMatrix {
    PolicyMatrix::new(DEFAULT_POLICY_RULES, NovaPolicyDecision::Deny)
}

pub fn evaluate_policy(request: NovaPolicyRequest) -> NovaPolicyDecision {
    if request.subject_service == NovaServiceId::POLICYD {
        return NovaPolicyDecision::Allow;
    }

    default_policy_matrix().decide(request)
}

const fn policy_scope_matches(rule_scope: NovaPolicyScope, request_scope: NovaPolicyScope) -> bool {
    match (rule_scope, request_scope) {
        (NovaPolicyScope::System, NovaPolicyScope::System) => true,
        (NovaPolicyScope::Scene(lhs), NovaPolicyScope::Scene(rhs)) => lhs.0 == rhs.0,
        (NovaPolicyScope::Agent(lhs), NovaPolicyScope::Agent(rhs)) => lhs.0 == rhs.0,
        (NovaPolicyScope::App(lhs), NovaPolicyScope::App(rhs)) => lhs.0 == rhs.0,
        _ => false,
    }
}
