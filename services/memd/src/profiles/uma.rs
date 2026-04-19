use crate::MemoryProfile;
use nova_fabric::{MemoryPoolKind, MemoryTopologyClass, PlatformClass};

const POOLS: [MemoryPoolKind; 3] = [
    MemoryPoolKind::SysCoherent,
    MemoryPoolKind::UmaAccelVisible,
    MemoryPoolKind::StagingIo,
];

pub struct UmaProfile;

impl MemoryProfile for UmaProfile {
    fn profile_name(&self) -> &'static str {
        "uma"
    }

    fn platform_class(&self) -> PlatformClass {
        PlatformClass::SparkUma
    }

    fn memory_topology(&self) -> MemoryTopologyClass {
        MemoryTopologyClass::Uma
    }

    fn supported_pools(&self) -> &'static [MemoryPoolKind] {
        &POOLS
    }
}
