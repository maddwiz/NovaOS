use crate::AccelBackend;
use nova_fabric::{AccelSeedV1, FabricCapabilityFlags, PlatformClass, QueueClass};

const CPU_QUEUE_CLASSES: [QueueClass; 2] = [QueueClass::Latency, QueueClass::Batch];

pub struct CpuBackend;

impl AccelBackend for CpuBackend {
    fn backend_name(&self) -> &'static str {
        "cpu"
    }

    fn platform_class(&self) -> PlatformClass {
        PlatformClass::Unknown
    }

    fn capability_flags(&self) -> FabricCapabilityFlags {
        FabricCapabilityFlags::empty()
    }

    fn supported_queue_classes(&self) -> &'static [QueueClass] {
        &CPU_QUEUE_CLASSES
    }

    fn supports_seed(&self, _seed: &AccelSeedV1) -> bool {
        false
    }
}
