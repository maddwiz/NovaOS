#![no_std]

use bitflags::bitflags;
use core::mem::size_of;

#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CpuArchitecture {
    Unknown = 0,
    Arm64 = 1,
    X86_64 = 2,
}

#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlatformClass {
    Unknown = 0,
    SparkUma = 1,
    PcieSingle = 2,
    PcieMulti = 3,
    FabricPartitioned = 4,
}

#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryTopologyClass {
    Unknown = 0,
    Uma = 1,
    Discrete = 2,
    Nvlink = 3,
    Mig = 4,
}

#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccelTransport {
    Unknown = 0,
    Integrated = 1,
    Platform = 2,
    Pci = 3,
    Fabric = 4,
}

#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccelTopologyHint {
    Unknown = 0,
    Uma = 1,
    Discrete = 2,
    Partitionable = 3,
    Linked = 4,
}

#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryPoolKind {
    SysCoherent = 1,
    UmaAccelVisible = 2,
    HostPinned = 3,
    GpuLocal = 4,
    PeerFabric = 5,
    PartitionLocal = 6,
    StagingIo = 7,
}

#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueueClass {
    Latency = 1,
    Batch = 2,
    Copy = 3,
    Maintenance = 4,
    LowPriBackground = 5,
}

bitflags! {
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct FabricCapabilityFlags: u64 {
        const INTEGRATED_ACCEL = 1 << 0;
        const DISCRETE_ACCEL = 1 << 1;
        const UMA_COHERENT = 1 << 2;
        const LOCAL_DEVICE_MEMORY = 1 << 3;
        const PINNED_HOST_MEMORY = 1 << 4;
        const PEER_TO_PEER_LINK = 1 << 5;
        const PARTITIONING = 1 << 6;
        const MIG_STYLE_PARTITIONS = 1 << 7;
        const COPY_ENGINES = 1 << 8;
        const PREEMPTION_LEVELS = 1 << 9;
        const SHARED_VA = 1 << 10;
        const BAR_ACCESS = 1 << 11;
        const DMA_TO_ACCEL = 1 << 12;
        const ACCEL_TO_DMA = 1 << 13;
        const DISPLAY_CAPABLE = 1 << 14;
        const HEADLESS_ONLY = 1 << 15;
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AccelMmioWindowV1 {
    pub base: u64,
    pub len: u64,
    pub flags: u32,
    pub reserved0: u32,
}

impl AccelMmioWindowV1 {
    pub const fn empty() -> Self {
        Self {
            base: 0,
            len: 0,
            flags: 0,
            reserved0: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AccelSeedV1 {
    pub vendor_id: u16,
    pub device_id: u16,
    pub class_code: u32,
    pub transport: AccelTransport,
    pub topology_hint: AccelTopologyHint,
    pub memory_topology: MemoryTopologyClass,
    pub mmio_window_count: u8,
    pub interrupt_hint: u8,
    pub reserved0: u16,
    pub raw_table_ptr: u64,
    pub raw_table_len: u32,
    pub reserved1: u32,
    pub mmio_windows: [AccelMmioWindowV1; 4],
}

impl AccelSeedV1 {
    pub const fn empty() -> Self {
        Self {
            vendor_id: 0,
            device_id: 0,
            class_code: 0,
            transport: AccelTransport::Unknown,
            topology_hint: AccelTopologyHint::Unknown,
            memory_topology: MemoryTopologyClass::Unknown,
            mmio_window_count: 0,
            interrupt_hint: 0,
            reserved0: 0,
            raw_table_ptr: 0,
            raw_table_len: 0,
            reserved1: 0,
            mmio_windows: [AccelMmioWindowV1::empty(); 4],
        }
    }

    pub const fn platform_ready(&self) -> bool {
        self.transport as u16 != AccelTransport::Unknown as u16
            && self.topology_hint as u16 != AccelTopologyHint::Unknown as u16
    }
}

const _: [(); 136] = [(); size_of::<AccelSeedV1>()];

#[cfg(test)]
mod tests {
    use super::{
        AccelSeedV1, AccelTopologyHint, AccelTransport, CpuArchitecture, FabricCapabilityFlags,
        MemoryPoolKind, MemoryTopologyClass, PlatformClass, QueueClass,
    };
    use core::mem::size_of;

    #[test]
    fn accel_seed_layout_is_stable() {
        assert_eq!(size_of::<AccelSeedV1>(), 136);
    }

    #[test]
    fn fabric_enums_match_portability_plan() {
        assert_eq!(CpuArchitecture::Arm64 as u16, 1);
        assert_eq!(CpuArchitecture::X86_64 as u16, 2);
        assert_eq!(PlatformClass::SparkUma as u16, 1);
        assert_eq!(PlatformClass::FabricPartitioned as u16, 4);
        assert_eq!(AccelTransport::Pci as u16, 3);
        assert_eq!(AccelTopologyHint::Partitionable as u16, 3);
        assert_eq!(MemoryTopologyClass::Mig as u16, 4);
        assert_eq!(MemoryPoolKind::GpuLocal as u16, 4);
        assert_eq!(QueueClass::Copy as u16, 3);
    }

    #[test]
    fn accel_seed_requires_non_unknown_transport_and_topology() {
        let seed = AccelSeedV1::empty();
        assert!(!seed.platform_ready());

        let mut seed = AccelSeedV1::empty();
        seed.transport = AccelTransport::Integrated;
        seed.topology_hint = AccelTopologyHint::Uma;
        assert!(seed.platform_ready());
    }

    #[test]
    fn capability_flags_cover_fabric_contract_baseline() {
        let caps = FabricCapabilityFlags::INTEGRATED_ACCEL
            | FabricCapabilityFlags::UMA_COHERENT
            | FabricCapabilityFlags::SHARED_VA;

        assert!(caps.contains(FabricCapabilityFlags::INTEGRATED_ACCEL));
        assert!(caps.contains(FabricCapabilityFlags::UMA_COHERENT));
        assert!(!caps.contains(FabricCapabilityFlags::DISCRETE_ACCEL));
    }
}
