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
        self.decide_with_audit(request, 0).decision
    }

    pub fn decide_with_audit(self, request: NovaPolicyRequest, sequence: u64) -> PolicyAuditRecord {
        let mut rule_index = 0;
        for rule in self.rules {
            if rule.matches(request) {
                return PolicyAuditRecord::from_rule(sequence, request, rule.decision, rule_index);
            }
            rule_index = rule_index.saturating_add(1);
        }
        PolicyAuditRecord::from_default(sequence, request, self.default_decision)
    }
}

pub const POLICY_AUDIT_NO_RULE: u16 = u16::MAX;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum PolicyDecisionSource {
    PolicydSelf = 1,
    MatrixRule = 2,
    MatrixDefault = 3,
}

impl PolicyDecisionSource {
    pub const fn label(self) -> &'static str {
        match self {
            Self::PolicydSelf => "policyd-self",
            Self::MatrixRule => "matrix-rule",
            Self::MatrixDefault => "matrix-default",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PolicyAuditRecord {
    pub sequence: u64,
    pub request: NovaPolicyRequest,
    pub decision: NovaPolicyDecision,
    pub source: PolicyDecisionSource,
    pub matched_rule_index: u16,
}

impl PolicyAuditRecord {
    pub const fn new(
        sequence: u64,
        request: NovaPolicyRequest,
        decision: NovaPolicyDecision,
        source: PolicyDecisionSource,
        matched_rule_index: u16,
    ) -> Self {
        Self {
            sequence,
            request,
            decision,
            source,
            matched_rule_index,
        }
    }

    pub const fn from_self_policy(sequence: u64, request: NovaPolicyRequest) -> Self {
        Self::new(
            sequence,
            request,
            NovaPolicyDecision::Allow,
            PolicyDecisionSource::PolicydSelf,
            POLICY_AUDIT_NO_RULE,
        )
    }

    pub const fn from_rule(
        sequence: u64,
        request: NovaPolicyRequest,
        decision: NovaPolicyDecision,
        matched_rule_index: u16,
    ) -> Self {
        Self::new(
            sequence,
            request,
            decision,
            PolicyDecisionSource::MatrixRule,
            matched_rule_index,
        )
    }

    pub const fn from_default(
        sequence: u64,
        request: NovaPolicyRequest,
        decision: NovaPolicyDecision,
    ) -> Self {
        Self::new(
            sequence,
            request,
            decision,
            PolicyDecisionSource::MatrixDefault,
            POLICY_AUDIT_NO_RULE,
        )
    }

    pub const fn matched_rule(self) -> bool {
        self.matched_rule_index != POLICY_AUDIT_NO_RULE
    }

    pub const fn allowed(self) -> bool {
        matches!(self.decision, NovaPolicyDecision::Allow)
    }

    pub const fn requires_approval(self) -> bool {
        matches!(self.decision, NovaPolicyDecision::Ask)
    }
}

const DEFAULT_POLICY_RULES: &[PolicyRule] = &[
    PolicyRule::new(
        NovaPolicyAction::LaunchService,
        NovaPolicyScope::Service(NovaServiceId::POLICYD),
        NovaPolicyDecision::Allow,
    ),
    PolicyRule::new(
        NovaPolicyAction::LaunchService,
        NovaPolicyScope::Service(NovaServiceId::AGENTD),
        NovaPolicyDecision::Allow,
    ),
    PolicyRule::new(
        NovaPolicyAction::LaunchService,
        NovaPolicyScope::Service(NovaServiceId::MEMD),
        NovaPolicyDecision::Allow,
    ),
    PolicyRule::new(
        NovaPolicyAction::LaunchService,
        NovaPolicyScope::Service(NovaServiceId::ACCELD),
        NovaPolicyDecision::Allow,
    ),
    PolicyRule::new(
        NovaPolicyAction::LaunchService,
        NovaPolicyScope::Service(NovaServiceId::INTENTD),
        NovaPolicyDecision::Allow,
    ),
    PolicyRule::new(
        NovaPolicyAction::LaunchService,
        NovaPolicyScope::Service(NovaServiceId::SCENED),
        NovaPolicyDecision::Allow,
    ),
    PolicyRule::new(
        NovaPolicyAction::LaunchService,
        NovaPolicyScope::Service(NovaServiceId::APPBRIDGED),
        NovaPolicyDecision::Allow,
    ),
    PolicyRule::new(
        NovaPolicyAction::LaunchService,
        NovaPolicyScope::Service(NovaServiceId::SHELLD),
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
    evaluate_policy_with_audit(request, 0).decision
}

pub fn evaluate_policy_with_audit(request: NovaPolicyRequest, sequence: u64) -> PolicyAuditRecord {
    if request.subject_service == NovaServiceId::POLICYD {
        return PolicyAuditRecord::from_self_policy(sequence, request);
    }

    default_policy_matrix().decide_with_audit(request, sequence)
}

const fn policy_scope_matches(rule_scope: NovaPolicyScope, request_scope: NovaPolicyScope) -> bool {
    match (rule_scope, request_scope) {
        (NovaPolicyScope::System, _) => true,
        (NovaPolicyScope::Service(lhs), NovaPolicyScope::Service(rhs)) => lhs.0 == rhs.0,
        (NovaPolicyScope::Scene(lhs), NovaPolicyScope::Scene(rhs)) => lhs.0 == rhs.0,
        (NovaPolicyScope::Agent(lhs), NovaPolicyScope::Agent(rhs)) => lhs.0 == rhs.0,
        (NovaPolicyScope::App(lhs), NovaPolicyScope::App(rhs)) => lhs.0 == rhs.0,
        _ => false,
    }
}
