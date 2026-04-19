use crate::AccelBackend;
use nova_fabric::{
    AccelSeedV1, AccelTopologyHint, AccelTransport, FabricCapabilityFlags, PlatformClass,
    QueueClass,
};

const GB10_QUEUE_CLASSES: [QueueClass; 4] = [
    QueueClass::Latency,
    QueueClass::Batch,
    QueueClass::Copy,
    QueueClass::Maintenance,
];

pub struct Gb10Backend;

impl AccelBackend for Gb10Backend {
    fn backend_name(&self) -> &'static str {
        "gb10"
    }

    fn platform_class(&self) -> PlatformClass {
        PlatformClass::SparkUma
    }

    fn capability_flags(&self) -> FabricCapabilityFlags {
        FabricCapabilityFlags::INTEGRATED_ACCEL
            | FabricCapabilityFlags::UMA_COHERENT
            | FabricCapabilityFlags::SHARED_VA
            | FabricCapabilityFlags::DMA_TO_ACCEL
            | FabricCapabilityFlags::ACCEL_TO_DMA
    }

    fn supported_queue_classes(&self) -> &'static [QueueClass] {
        &GB10_QUEUE_CLASSES
    }

    fn supports_seed(&self, seed: &AccelSeedV1) -> bool {
        seed.transport == AccelTransport::Integrated && seed.topology_hint == AccelTopologyHint::Uma
    }
}
