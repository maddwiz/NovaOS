use crate::MemoryProfile;
use nova_fabric::{MemoryPoolKind, MemoryTopologyClass, PlatformClass};

const POOLS: [MemoryPoolKind; 3] = [
    MemoryPoolKind::SysCoherent,
    MemoryPoolKind::HostPinned,
    MemoryPoolKind::GpuLocal,
];

pub struct DiscreteProfile;

impl MemoryProfile for DiscreteProfile {
    fn profile_name(&self) -> &'static str {
        "discrete"
    }

    fn platform_class(&self) -> PlatformClass {
        PlatformClass::PcieSingle
    }

    fn memory_topology(&self) -> MemoryTopologyClass {
        MemoryTopologyClass::Discrete
    }

    fn supported_pools(&self) -> &'static [MemoryPoolKind] {
        &POOLS
    }
}
