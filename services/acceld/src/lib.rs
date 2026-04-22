#![no_std]

pub mod backends;
pub mod dispatch;
pub mod launch;

pub use dispatch::{
    AccelDispatchPlan, AccelDispatchRequest, AccelDispatchStatus, plan_accel_dispatch,
};
pub use launch::{ACCELD_DESCRIPTOR, ACCELD_LAUNCH_SPEC};

use nova_fabric::{AccelSeedV1, FabricCapabilityFlags, PlatformClass, QueueClass};

pub trait AccelBackend {
    fn backend_name(&self) -> &'static str;
    fn platform_class(&self) -> PlatformClass;
    fn capability_flags(&self) -> FabricCapabilityFlags;
    fn supported_queue_classes(&self) -> &'static [QueueClass];
    fn supports_seed(&self, seed: &AccelSeedV1) -> bool;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackendDescriptor {
    pub name: &'static str,
    pub platform_class: PlatformClass,
    pub capability_flags: FabricCapabilityFlags,
    pub queue_class_count: usize,
}

pub fn describe_backend(backend: &dyn AccelBackend) -> BackendDescriptor {
    BackendDescriptor {
        name: backend.backend_name(),
        platform_class: backend.platform_class(),
        capability_flags: backend.capability_flags(),
        queue_class_count: backend.supported_queue_classes().len(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ACCELD_LAUNCH_SPEC, AccelBackend, AccelDispatchRequest, AccelDispatchStatus, backends,
        describe_backend, plan_accel_dispatch,
    };
    use nova_fabric::{AccelSeedV1, AccelTopologyHint, AccelTransport, PlatformClass, QueueClass};
    use nova_rt::NovaServiceId;

    #[test]
    fn cpu_backend_is_present_for_all_platforms() {
        let backend = backends::cpu::CpuBackend;
        let descriptor = describe_backend(&backend);
        assert_eq!(descriptor.name, "cpu");
        assert_eq!(descriptor.platform_class, PlatformClass::Unknown);
    }

    #[test]
    fn gb10_backend_filters_for_integrated_uma_seeds() {
        let backend = backends::gb10::Gb10Backend;
        let mut seed = AccelSeedV1::empty();
        seed.transport = AccelTransport::Integrated;
        seed.topology_hint = AccelTopologyHint::Uma;

        assert!(backend.supports_seed(&seed));

        seed.transport = AccelTransport::Pci;
        assert!(!backend.supports_seed(&seed));
    }

    #[test]
    fn launch_spec_identifies_accel_service() {
        assert_eq!(ACCELD_LAUNCH_SPEC.descriptor.id, NovaServiceId::ACCELD);
        assert!(ACCELD_LAUNCH_SPEC.is_valid());
    }

    #[test]
    fn dispatch_labels_are_stable_for_operator_reports() {
        assert_eq!(AccelDispatchStatus::Ready.label(), "ready");
        assert_eq!(AccelDispatchStatus::CpuFallback.label(), "cpu-fallback");
        assert_eq!(
            AccelDispatchStatus::MissingPlatformSeed.label(),
            "missing-platform-seed"
        );
        assert_eq!(
            AccelDispatchStatus::UnsupportedQueue.label(),
            "unsupported-queue"
        );
    }

    #[test]
    fn gb10_seed_routes_latency_queue_to_gb10_backend() {
        let cpu = backends::cpu::CpuBackend;
        let gb10 = backends::gb10::Gb10Backend;
        let rtx = backends::rtx::RtxBackend;
        let hopper = backends::hopper::HopperBackend;
        let backends: [&dyn AccelBackend; 4] = [&cpu, &gb10, &rtx, &hopper];
        let mut seed = AccelSeedV1::empty();
        seed.transport = AccelTransport::Integrated;
        seed.topology_hint = AccelTopologyHint::Uma;
        let request = AccelDispatchRequest::exact(QueueClass::Latency);

        let plan = plan_accel_dispatch(&backends, &seed, request);

        assert_eq!(plan.status, AccelDispatchStatus::Ready);
        assert_eq!(
            plan.selected_backend.map(|backend| backend.name),
            Some("gb10")
        );
        assert!(plan.seed_ready);
        assert!(plan.is_ready());
        assert!(!plan.used_cpu_fallback());
    }

    #[test]
    fn rtx_seed_routes_copy_queue_to_rtx_backend() {
        let cpu = backends::cpu::CpuBackend;
        let gb10 = backends::gb10::Gb10Backend;
        let rtx = backends::rtx::RtxBackend;
        let hopper = backends::hopper::HopperBackend;
        let backends: [&dyn AccelBackend; 4] = [&cpu, &gb10, &rtx, &hopper];
        let mut seed = AccelSeedV1::empty();
        seed.transport = AccelTransport::Pci;
        seed.topology_hint = AccelTopologyHint::Discrete;
        let request = AccelDispatchRequest::exact(QueueClass::Copy);

        let plan = plan_accel_dispatch(&backends, &seed, request);

        assert_eq!(plan.status, AccelDispatchStatus::Ready);
        assert_eq!(
            plan.selected_backend.map(|backend| backend.name),
            Some("rtx")
        );
    }

    #[test]
    fn hopper_partition_seed_routes_maintenance_queue_to_hopper_backend() {
        let cpu = backends::cpu::CpuBackend;
        let gb10 = backends::gb10::Gb10Backend;
        let rtx = backends::rtx::RtxBackend;
        let hopper = backends::hopper::HopperBackend;
        let backends: [&dyn AccelBackend; 4] = [&cpu, &gb10, &rtx, &hopper];
        let mut seed = AccelSeedV1::empty();
        seed.transport = AccelTransport::Fabric;
        seed.topology_hint = AccelTopologyHint::Partitionable;
        let request = AccelDispatchRequest::exact(QueueClass::Maintenance);

        let plan = plan_accel_dispatch(&backends, &seed, request);

        assert_eq!(plan.status, AccelDispatchStatus::Ready);
        assert_eq!(
            plan.selected_backend.map(|backend| backend.name),
            Some("hopper")
        );
    }

    #[test]
    fn cpu_fallback_handles_missing_seed_for_cpu_supported_queues() {
        let cpu = backends::cpu::CpuBackend;
        let gb10 = backends::gb10::Gb10Backend;
        let backends: [&dyn AccelBackend; 2] = [&cpu, &gb10];
        let seed = AccelSeedV1::empty();
        let request = AccelDispatchRequest::new(QueueClass::Batch, true);

        let plan = plan_accel_dispatch(&backends, &seed, request);

        assert_eq!(plan.status, AccelDispatchStatus::CpuFallback);
        assert_eq!(
            plan.selected_backend.map(|backend| backend.name),
            Some("cpu")
        );
        assert!(!plan.seed_ready);
        assert!(plan.used_cpu_fallback());
    }

    #[test]
    fn missing_seed_without_fallback_is_not_ready() {
        let cpu = backends::cpu::CpuBackend;
        let gb10 = backends::gb10::Gb10Backend;
        let backends: [&dyn AccelBackend; 2] = [&cpu, &gb10];
        let seed = AccelSeedV1::empty();
        let request = AccelDispatchRequest::exact(QueueClass::Latency);

        let plan = plan_accel_dispatch(&backends, &seed, request);

        assert_eq!(plan.status, AccelDispatchStatus::MissingPlatformSeed);
        assert_eq!(plan.selected_backend, None);
        assert!(!plan.is_ready());
    }

    #[test]
    fn unsupported_queue_rejects_matching_backend_when_no_fallback_can_cover_it() {
        let cpu = backends::cpu::CpuBackend;
        let gb10 = backends::gb10::Gb10Backend;
        let backends: [&dyn AccelBackend; 2] = [&cpu, &gb10];
        let mut seed = AccelSeedV1::empty();
        seed.transport = AccelTransport::Integrated;
        seed.topology_hint = AccelTopologyHint::Uma;
        let request = AccelDispatchRequest::new(QueueClass::LowPriBackground, true);

        let plan = plan_accel_dispatch(&backends, &seed, request);

        assert_eq!(plan.status, AccelDispatchStatus::UnsupportedQueue);
        assert_eq!(plan.selected_backend, None);
    }
}
