fn main() {
    let snapshot = novaos_initd::initd_boot_snapshot();
    println!(
        "initd services={} required={} healthy={}",
        snapshot.launch_service_count,
        snapshot.required_service_count,
        snapshot.healthy()
    );
}
