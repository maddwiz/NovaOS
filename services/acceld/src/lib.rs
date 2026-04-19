#![no_std]

pub mod backends;

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
    use super::{AccelBackend, backends, describe_backend};
    use nova_fabric::{AccelSeedV1, AccelTopologyHint, AccelTransport, PlatformClass};

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
}
