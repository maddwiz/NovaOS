#![no_std]

pub mod profiles;

use nova_fabric::{MemoryPoolKind, MemoryTopologyClass, PlatformClass};

pub trait MemoryProfile {
    fn profile_name(&self) -> &'static str;
    fn platform_class(&self) -> PlatformClass;
    fn memory_topology(&self) -> MemoryTopologyClass;
    fn supported_pools(&self) -> &'static [MemoryPoolKind];
}

#[cfg(test)]
mod tests {
    use super::{MemoryProfile, profiles};
    use nova_fabric::{MemoryPoolKind, MemoryTopologyClass, PlatformClass};

    #[test]
    fn uma_profile_is_spark_focused() {
        let profile = profiles::uma::UmaProfile;
        assert_eq!(profile.platform_class(), PlatformClass::SparkUma);
        assert_eq!(profile.memory_topology(), MemoryTopologyClass::Uma);
        assert!(
            profile
                .supported_pools()
                .contains(&MemoryPoolKind::UmaAccelVisible)
        );
    }

    #[test]
    fn mig_profile_is_partition_focused() {
        let profile = profiles::mig::MigProfile;
        assert_eq!(profile.memory_topology(), MemoryTopologyClass::Mig);
        assert!(
            profile
                .supported_pools()
                .contains(&MemoryPoolKind::PartitionLocal)
        );
    }
}
