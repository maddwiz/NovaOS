use crate::{
    core_launch_plan, core_launch_table, initd_boot_snapshot, initd_boot_status_page,
    initd_kernel_launch_plan_page, initd_runtime_report,
};
use nova_rt::{
    NovaPolicyDecision, NovaPolicyScope, NovaServiceBindingState, NovaServiceId,
    NovaServiceLaunchSpec, NovaServiceLaunchStatus, NovaServiceState,
};

const SERVICE_OWNED_LAUNCH_SPECS: &[NovaServiceLaunchSpec] = &[
    novaos_policyd::POLICYD_LAUNCH_SPEC,
    novaos_agentd::AGENTD_LAUNCH_SPEC,
    novaos_memd::MEMD_LAUNCH_SPEC,
    novaos_acceld::ACCELD_LAUNCH_SPEC,
    novaos_intentd::INTENTD_LAUNCH_SPEC,
    novaos_scened::SCENED_LAUNCH_SPEC,
    novaos_appbridged::APPBRIDGED_LAUNCH_SPEC,
    novaos_shelld::SHELLD_LAUNCH_SPEC,
];

#[test]
fn init_launch_table_starts_policy_before_agents_and_intents() {
    let table = core_launch_table();

    assert_eq!(
        table.launch_request(0).expect("policyd").target,
        NovaServiceId::POLICYD
    );
    assert_eq!(
        table.launch_request(1).expect("agentd").target,
        NovaServiceId::AGENTD
    );
    assert_eq!(
        table.launch_request(4).expect("intentd").target,
        NovaServiceId::INTENTD
    );
}

#[test]
fn init_snapshot_reports_runtime_health() {
    let snapshot = initd_boot_snapshot();

    assert_eq!(snapshot.registered_service.id, NovaServiceId::INITD);
    assert!(snapshot.healthy());
    assert!(snapshot.launch_service_count >= 7);
}

#[test]
fn init_status_page_reports_required_services_running() {
    let status_page = initd_boot_status_page();

    assert_eq!(status_page.registered_service.id, NovaServiceId::INITD);
    assert!(status_page.healthy());
    assert_eq!(status_page.running_required_service_count(), 7);
    assert_eq!(
        status_page
            .status_for(NovaServiceId::POLICYD)
            .expect("policyd")
            .state,
        NovaServiceState::Running
    );
}

#[test]
fn init_status_page_keeps_optional_shell_deferred() {
    let status = initd_boot_status_page()
        .status_for(NovaServiceId::SHELLD)
        .expect("shelld");

    assert_eq!(status.state, NovaServiceState::NotStarted);
    assert_eq!(status.last_result.status, NovaServiceLaunchStatus::Deferred);
}

#[test]
fn core_launch_specs_match_static_launch_order() {
    let table = core_launch_table();
    let plan = core_launch_plan();

    assert_eq!(table.service_count(), plan.service_count());
    for (index, service) in table.services.iter().enumerate() {
        assert_eq!(service.id, plan.specs[index].descriptor.id);
    }
}

#[test]
fn core_launch_specs_are_service_owned() {
    let plan = core_launch_plan();

    assert_eq!(plan.specs, SERVICE_OWNED_LAUNCH_SPECS);
}

#[test]
fn core_launch_plan_resolves_policyd_request_and_context() {
    let plan = core_launch_plan();
    let request = plan
        .launch_request_for(NovaServiceId::POLICYD)
        .expect("policyd request");
    let policy = plan
        .launch_policy_decision_for(NovaServiceId::POLICYD)
        .expect("policyd launch policy");
    let policy_request = plan
        .launch_policy_request_for(NovaServiceId::POLICYD)
        .expect("policyd policy request");
    let spec = plan.spec_for(NovaServiceId::POLICYD).expect("policyd spec");
    let context = spec.bootstrap_context_v1().expect("bootstrap context");

    assert_eq!(request.requester, NovaServiceId::INITD);
    assert_eq!(request.target, NovaServiceId::POLICYD);
    assert_eq!(
        policy_request.scope,
        NovaPolicyScope::Service(NovaServiceId::POLICYD)
    );
    assert_eq!(policy, NovaPolicyDecision::Allow);
    assert_eq!(context.service_name(), "policyd");
    assert_eq!(context.endpoint_slots, 1);
    assert_eq!(context.shared_memory_regions, 1);
}

#[test]
fn core_launch_plan_keeps_optional_shell_model_only() {
    let status = initd_boot_status_page()
        .status_for(NovaServiceId::SHELLD)
        .expect("shelld status");
    let kernel_plan = initd_kernel_launch_plan_page()
        .plan_for(NovaServiceId::SHELLD)
        .expect("shelld kernel plan");

    assert_eq!(status.last_result.status, NovaServiceLaunchStatus::Deferred);
    assert_eq!(
        kernel_plan.binding.state,
        NovaServiceBindingState::ModelOnly
    );
    assert!(!kernel_plan.binding.has_kernel_objects());
}

#[test]
fn runtime_report_joins_status_and_kernel_binding() {
    let report = initd_runtime_report();
    let service = report
        .service_report_for(NovaServiceId::POLICYD)
        .expect("policyd report");

    assert!(report.healthy());
    assert_eq!(report.allowed_service_count(), 8);
    assert_eq!(report.approval_required_service_count(), 0);
    assert_eq!(report.denied_service_count(), 0);
    assert_eq!(service.descriptor.id, NovaServiceId::POLICYD);
    assert_eq!(service.launch_request.requester, NovaServiceId::INITD);
    assert_eq!(service.launch_request.target, NovaServiceId::POLICYD);
    assert_eq!(service.state, NovaServiceState::Running);
    assert_eq!(service.launch_status, NovaServiceLaunchStatus::Started);
    assert_eq!(service.policy_decision(), NovaPolicyDecision::Allow);
    assert_eq!(service.policy_audit.sequence, 1);
    assert_eq!(service.policy_audit.matched_rule_index, 0);
    assert!(service.policy_allows_launch());
    assert!(!service.policy_requires_approval());
    assert_eq!(service.kernel_binding.task.0, 0x1001);
    assert_eq!(service.kernel_binding.control_endpoint.0, 0x2001);
    assert!(service.has_kernel_objects());
    assert!(!service.can_publish_kernel_health());
}

#[test]
fn runtime_report_keeps_optional_shell_deferred_and_model_only() {
    let service = initd_runtime_report()
        .service_report_for(NovaServiceId::SHELLD)
        .expect("shelld report");

    assert_eq!(service.state.label(), "not-started");
    assert_eq!(service.launch_status.label(), "deferred");
    assert_eq!(service.policy_decision().label(), "allow");
    assert_eq!(service.policy_audit.sequence, 8);
    assert_eq!(service.policy_audit.matched_rule_index, 7);
    assert_eq!(service.kernel_binding.state.label(), "model-only");
    assert!(!service.descriptor.required);
    assert!(service.is_required_healthy());
    assert!(!service.has_kernel_objects());
}

#[test]
fn core_launch_plan_validates_unique_ids_and_order() {
    let plan = core_launch_plan();
    let kernel_plan = initd_kernel_launch_plan_page();

    assert!(plan.validate());
    assert_eq!(plan.required_service_count(), 7);
    assert!(kernel_plan.ready_for_kernel_handoff());
    assert_eq!(kernel_plan.planned_required_service_count(), 7);
    assert_eq!(kernel_plan.kernel_backed_service_count(), 0);
}
