use crate::{core_launch_table, initd_boot_snapshot, initd_boot_status_page};
use nova_rt::{NovaServiceId, NovaServiceLaunchStatus, NovaServiceState};

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
