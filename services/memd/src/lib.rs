#![no_std]

pub mod launch;
pub mod placement;
pub mod profiles;

pub use launch::{MEMD_DESCRIPTOR, MEMD_LAUNCH_SPEC, MEMD_PAYLOAD_SPEC};
pub use placement::{
    MemoryPlacementGoal, MemoryPlacementPlan, MemoryPlacementRequest, MemoryPlacementStatus,
    plan_memory_placement,
};

use nova_fabric::{MemoryPoolKind, MemoryTopologyClass, PlatformClass};

pub trait MemoryProfile {
    fn profile_name(&self) -> &'static str;
    fn platform_class(&self) -> PlatformClass;
    fn memory_topology(&self) -> MemoryTopologyClass;
    fn supported_pools(&self) -> &'static [MemoryPoolKind];
}

#[cfg(test)]
mod tests {
    use super::{
        MEMD_LAUNCH_SPEC, MEMD_PAYLOAD_SPEC, MemoryPlacementGoal, MemoryPlacementRequest,
        MemoryPlacementStatus, MemoryProfile, plan_memory_placement, profiles,
    };
    use nova_fabric::{MemoryPoolKind, MemoryTopologyClass, PlatformClass};
    use nova_rt::{NovaPayloadEntryAbi, NovaPayloadKind, NovaServiceId};

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

    #[test]
    fn launch_spec_identifies_memory_service() {
        assert_eq!(MEMD_LAUNCH_SPEC.descriptor.id, NovaServiceId::MEMD);
        assert!(MEMD_LAUNCH_SPEC.is_valid());
        assert_eq!(MEMD_LAUNCH_SPEC.artifact, Some(MEMD_PAYLOAD_SPEC));
        assert_eq!(MEMD_PAYLOAD_SPEC.image_stem, "memd-payload");
        assert_eq!(MEMD_PAYLOAD_SPEC.payload_kind, NovaPayloadKind::Service);
        assert_eq!(
            MEMD_PAYLOAD_SPEC.entry_abi,
            NovaPayloadEntryAbi::BootstrapTaskV1
        );
    }

    #[test]
    fn placement_labels_are_stable_for_operator_reports() {
        assert_eq!(
            MemoryPlacementGoal::AcceleratorVisible.label(),
            "accelerator-visible"
        );
        assert_eq!(
            MemoryPlacementGoal::PartitionLocal.label(),
            "partition-local"
        );
        assert_eq!(MemoryPlacementStatus::Ready.label(), "ready");
        assert_eq!(
            MemoryPlacementStatus::UnsupportedGoal.label(),
            "unsupported-goal"
        );
    }

    #[test]
    fn uma_profile_places_accelerator_visible_memory_directly() {
        let profile = profiles::uma::UmaProfile;
        let request = MemoryPlacementRequest::exact(4096, MemoryPlacementGoal::AcceleratorVisible);

        let plan = plan_memory_placement(&profile, request);

        assert_eq!(plan.profile_name, "uma");
        assert_eq!(plan.topology, MemoryTopologyClass::Uma);
        assert_eq!(plan.selected_pool, Some(MemoryPoolKind::UmaAccelVisible));
        assert_eq!(plan.status, MemoryPlacementStatus::Ready);
        assert!(plan.is_ready());
        assert!(!plan.used_fallback());
    }

    #[test]
    fn discrete_profile_falls_back_to_host_pinned_for_accelerator_visibility() {
        let profile = profiles::discrete::DiscreteProfile;
        let request =
            MemoryPlacementRequest::new(4096, MemoryPlacementGoal::AcceleratorVisible, true);

        let plan = plan_memory_placement(&profile, request);

        assert_eq!(plan.selected_pool, Some(MemoryPoolKind::HostPinned));
        assert_eq!(plan.status, MemoryPlacementStatus::Fallback);
        assert!(plan.is_ready());
        assert!(plan.used_fallback());
    }

    #[test]
    fn exact_placement_rejects_unsupported_goal_without_fallback() {
        let profile = profiles::discrete::DiscreteProfile;
        let request = MemoryPlacementRequest::exact(4096, MemoryPlacementGoal::AcceleratorVisible);

        let plan = plan_memory_placement(&profile, request);

        assert_eq!(plan.selected_pool, None);
        assert_eq!(plan.status, MemoryPlacementStatus::UnsupportedGoal);
        assert!(!plan.is_ready());
    }

    #[test]
    fn nvlink_profile_places_peer_fabric_requests() {
        let profile = profiles::nvlink::NvlinkProfile;
        let request = MemoryPlacementRequest::exact(8192, MemoryPlacementGoal::PeerFabric);

        let plan = plan_memory_placement(&profile, request);

        assert_eq!(plan.selected_pool, Some(MemoryPoolKind::PeerFabric));
        assert_eq!(plan.status, MemoryPlacementStatus::Ready);
    }

    #[test]
    fn mig_profile_places_partition_local_requests() {
        let profile = profiles::mig::MigProfile;
        let request = MemoryPlacementRequest::exact(8192, MemoryPlacementGoal::PartitionLocal);

        let plan = plan_memory_placement(&profile, request);

        assert_eq!(plan.selected_pool, Some(MemoryPoolKind::PartitionLocal));
        assert_eq!(plan.status, MemoryPlacementStatus::Ready);
    }

    #[test]
    fn zero_byte_requests_are_not_placement_ready() {
        let profile = profiles::uma::UmaProfile;
        let request = MemoryPlacementRequest::new(0, MemoryPlacementGoal::SystemCoherent, true);

        let plan = plan_memory_placement(&profile, request);

        assert_eq!(plan.selected_pool, None);
        assert_eq!(plan.status, MemoryPlacementStatus::EmptyRequest);
        assert!(!plan.is_ready());
    }
}
