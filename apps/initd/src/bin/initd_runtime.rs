fn main() {
    let snapshot = novaos_initd::initd_boot_snapshot();
    let report = novaos_initd::initd_runtime_report();

    println!(
        "initd services={} required={} healthy={} generation={} kernel_planned_required={} kernel_backed={} policy_allowed={} policy_ask={} policy_denied={}",
        snapshot.launch_service_count,
        snapshot.required_service_count,
        report.healthy(),
        snapshot.health_generation,
        report.kernel_plan_page.planned_required_service_count(),
        report.kernel_plan_page.kernel_backed_service_count(),
        report.allowed_service_count(),
        report.approval_required_service_count(),
        report.denied_service_count(),
    );

    for index in 0..report.service_count() {
        let service = report
            .service_report(index)
            .expect("initd service report must match status page");
        let (artifact_name, artifact_embedded) = match service.artifact {
            Some(artifact) => (artifact.image_stem, artifact.embedded_in_init_capsule),
            None => ("none", false),
        };
        println!(
            "service name={} kind={} required={} order={} requester=0x{:x} target=0x{:x} scene={} artifact={} artifact_embedded={} state={} launch={} detail={} policy={} policy_source={} policy_rule={} policy_seq={} binding={} task=0x{:x} endpoint=0x{:x} shm=0x{:x}",
            service.descriptor.name,
            service.descriptor.kind.label(),
            service.descriptor.required,
            service.descriptor.launch_order,
            service.launch_request.requester.0,
            service.launch_request.target.0,
            service.launch_request.scene.0,
            artifact_name,
            artifact_embedded,
            service.state.label(),
            service.launch_status.label(),
            service.launch_detail,
            service.policy_decision().label(),
            service.policy_audit.source.label(),
            service.policy_audit.matched_rule_index,
            service.policy_audit.sequence,
            service.kernel_binding.state.label(),
            service.kernel_binding.task.0,
            service.kernel_binding.control_endpoint.0,
            service.kernel_binding.shared_memory_region.0,
        );
    }
}
