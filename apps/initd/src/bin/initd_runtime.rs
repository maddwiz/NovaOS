fn main() {
    let snapshot = novaos_initd::initd_boot_snapshot();
    let report = novaos_initd::initd_runtime_report();

    println!(
        "initd services={} required={} healthy={} generation={} kernel_planned_required={} kernel_backed={}",
        snapshot.launch_service_count,
        snapshot.required_service_count,
        report.healthy(),
        snapshot.health_generation,
        report.kernel_plan_page.planned_required_service_count(),
        report.kernel_plan_page.kernel_backed_service_count(),
    );

    for index in 0..report.service_count() {
        let service = report
            .service_report(index)
            .expect("initd service report must match status page");
        println!(
            "service name={} kind={} required={} order={} state={} launch={} detail={} binding={} task=0x{:x} endpoint=0x{:x} shm=0x{:x}",
            service.descriptor.name,
            service.descriptor.kind.label(),
            service.descriptor.required,
            service.descriptor.launch_order,
            service.state.label(),
            service.launch_status.label(),
            service.launch_detail,
            service.kernel_binding.state.label(),
            service.kernel_binding.task.0,
            service.kernel_binding.control_endpoint.0,
            service.kernel_binding.shared_memory_region.0,
        );
    }
}
