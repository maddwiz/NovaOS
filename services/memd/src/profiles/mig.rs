use crate::MemoryProfile;
use nova_fabric::{MemoryPoolKind, MemoryTopologyClass, PlatformClass};

const POOLS: [MemoryPoolKind; 4] = [
    MemoryPoolKind::SysCoherent,
    MemoryPoolKind::GpuLocal,
    MemoryPoolKind::PartitionLocal,
    MemoryPoolKind::PeerFabric,
];

pub struct MigProfile;

impl MemoryProfile for MigProfile {
    fn profile_name(&self) -> &'static str {
        "mig"
    }

    fn platform_class(&self) -> PlatformClass {
        PlatformClass::FabricPartitioned
    }

    fn memory_topology(&self) -> MemoryTopologyClass {
        MemoryTopologyClass::Mig
    }

    fn supported_pools(&self) -> &'static [MemoryPoolKind] {
        &POOLS
    }
}
