use crate::MemoryProfile;
use nova_fabric::{MemoryPoolKind, MemoryTopologyClass, PlatformClass};

const POOLS: [MemoryPoolKind; 4] = [
    MemoryPoolKind::SysCoherent,
    MemoryPoolKind::GpuLocal,
    MemoryPoolKind::PeerFabric,
    MemoryPoolKind::HostPinned,
];

pub struct NvlinkProfile;

impl MemoryProfile for NvlinkProfile {
    fn profile_name(&self) -> &'static str {
        "nvlink"
    }

    fn platform_class(&self) -> PlatformClass {
        PlatformClass::PcieMulti
    }

    fn memory_topology(&self) -> MemoryTopologyClass {
        MemoryTopologyClass::Nvlink
    }

    fn supported_pools(&self) -> &'static [MemoryPoolKind] {
        &POOLS
    }
}
