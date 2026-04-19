use crate::AccelBackend;
use nova_fabric::{
    AccelSeedV1, AccelTopologyHint, AccelTransport, FabricCapabilityFlags, PlatformClass,
    QueueClass,
};

const HOPPER_QUEUE_CLASSES: [QueueClass; 4] = [
    QueueClass::Batch,
    QueueClass::Copy,
    QueueClass::Maintenance,
    QueueClass::LowPriBackground,
];

pub struct HopperBackend;

impl AccelBackend for HopperBackend {
    fn backend_name(&self) -> &'static str {
        "hopper"
    }

    fn platform_class(&self) -> PlatformClass {
        PlatformClass::FabricPartitioned
    }

    fn capability_flags(&self) -> FabricCapabilityFlags {
        FabricCapabilityFlags::DISCRETE_ACCEL
            | FabricCapabilityFlags::LOCAL_DEVICE_MEMORY
            | FabricCapabilityFlags::PEER_TO_PEER_LINK
            | FabricCapabilityFlags::PARTITIONING
            | FabricCapabilityFlags::MIG_STYLE_PARTITIONS
            | FabricCapabilityFlags::COPY_ENGINES
    }

    fn supported_queue_classes(&self) -> &'static [QueueClass] {
        &HOPPER_QUEUE_CLASSES
    }

    fn supports_seed(&self, seed: &AccelSeedV1) -> bool {
        seed.transport == AccelTransport::Fabric
            || seed.topology_hint == AccelTopologyHint::Partitionable
    }
}
