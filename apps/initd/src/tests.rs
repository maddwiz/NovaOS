use crate::{core_launch_table, initd_boot_snapshot};
use nova_rt::NovaServiceId;

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
