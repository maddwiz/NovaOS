use crate::AccelBackend;
use nova_fabric::{
    AccelSeedV1, AccelTopologyHint, AccelTransport, FabricCapabilityFlags, PlatformClass,
    QueueClass,
};

const RTX_QUEUE_CLASSES: [QueueClass; 4] = [
    QueueClass::Latency,
    QueueClass::Batch,
    QueueClass::Copy,
    QueueClass::LowPriBackground,
];

pub struct RtxBackend;

impl AccelBackend for RtxBackend {
    fn backend_name(&self) -> &'static str {
        "rtx"
    }

    fn platform_class(&self) -> PlatformClass {
        PlatformClass::PcieSingle
    }

    fn capability_flags(&self) -> FabricCapabilityFlags {
        FabricCapabilityFlags::DISCRETE_ACCEL
            | FabricCapabilityFlags::LOCAL_DEVICE_MEMORY
            | FabricCapabilityFlags::PINNED_HOST_MEMORY
            | FabricCapabilityFlags::BAR_ACCESS
            | FabricCapabilityFlags::COPY_ENGINES
    }

    fn supported_queue_classes(&self) -> &'static [QueueClass] {
        &RTX_QUEUE_CLASSES
    }

    fn supports_seed(&self, seed: &AccelSeedV1) -> bool {
        seed.transport == AccelTransport::Pci && seed.topology_hint == AccelTopologyHint::Discrete
    }
}
