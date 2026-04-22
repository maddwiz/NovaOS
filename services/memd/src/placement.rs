use crate::MemoryProfile;
use nova_fabric::{MemoryPoolKind, MemoryTopologyClass};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum MemoryPlacementGoal {
    SystemCoherent = 1,
    AcceleratorVisible = 2,
    DeviceLocal = 3,
    PeerFabric = 4,
    PartitionLocal = 5,
    StagingIo = 6,
}

impl MemoryPlacementGoal {
    pub const fn label(self) -> &'static str {
        match self {
            Self::SystemCoherent => "system-coherent",
            Self::AcceleratorVisible => "accelerator-visible",
            Self::DeviceLocal => "device-local",
            Self::PeerFabric => "peer-fabric",
            Self::PartitionLocal => "partition-local",
            Self::StagingIo => "staging-io",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryPlacementRequest {
    pub bytes: u64,
    pub goal: MemoryPlacementGoal,
    pub allow_fallback: bool,
}

impl MemoryPlacementRequest {
    pub const fn new(bytes: u64, goal: MemoryPlacementGoal, allow_fallback: bool) -> Self {
        Self {
            bytes,
            goal,
            allow_fallback,
        }
    }

    pub const fn exact(bytes: u64, goal: MemoryPlacementGoal) -> Self {
        Self::new(bytes, goal, false)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum MemoryPlacementStatus {
    Ready = 1,
    Fallback = 2,
    EmptyRequest = 3,
    UnsupportedGoal = 4,
}

impl MemoryPlacementStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Fallback => "fallback",
            Self::EmptyRequest => "empty-request",
            Self::UnsupportedGoal => "unsupported-goal",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryPlacementPlan {
    pub profile_name: &'static str,
    pub topology: MemoryTopologyClass,
    pub request: MemoryPlacementRequest,
    pub selected_pool: Option<MemoryPoolKind>,
    pub status: MemoryPlacementStatus,
}

impl MemoryPlacementPlan {
    pub const fn new(
        profile_name: &'static str,
        topology: MemoryTopologyClass,
        request: MemoryPlacementRequest,
        selected_pool: Option<MemoryPoolKind>,
        status: MemoryPlacementStatus,
    ) -> Self {
        Self {
            profile_name,
            topology,
            request,
            selected_pool,
            status,
        }
    }

    pub const fn is_ready(self) -> bool {
        matches!(
            self.status,
            MemoryPlacementStatus::Ready | MemoryPlacementStatus::Fallback
        )
    }

    pub const fn used_fallback(self) -> bool {
        matches!(self.status, MemoryPlacementStatus::Fallback)
    }
}

pub fn plan_memory_placement(
    profile: &dyn MemoryProfile,
    request: MemoryPlacementRequest,
) -> MemoryPlacementPlan {
    if request.bytes == 0 {
        return MemoryPlacementPlan::new(
            profile.profile_name(),
            profile.memory_topology(),
            request,
            None,
            MemoryPlacementStatus::EmptyRequest,
        );
    }

    let primary = primary_pool(request.goal);
    if pool_supported(profile, primary) {
        return MemoryPlacementPlan::new(
            profile.profile_name(),
            profile.memory_topology(),
            request,
            Some(primary),
            MemoryPlacementStatus::Ready,
        );
    }

    if request.allow_fallback {
        for pool in fallback_pools(request.goal) {
            if pool_supported(profile, *pool) {
                return MemoryPlacementPlan::new(
                    profile.profile_name(),
                    profile.memory_topology(),
                    request,
                    Some(*pool),
                    MemoryPlacementStatus::Fallback,
                );
            }
        }
    }

    MemoryPlacementPlan::new(
        profile.profile_name(),
        profile.memory_topology(),
        request,
        None,
        MemoryPlacementStatus::UnsupportedGoal,
    )
}

fn pool_supported(profile: &dyn MemoryProfile, pool: MemoryPoolKind) -> bool {
    profile.supported_pools().contains(&pool)
}

const fn primary_pool(goal: MemoryPlacementGoal) -> MemoryPoolKind {
    match goal {
        MemoryPlacementGoal::SystemCoherent => MemoryPoolKind::SysCoherent,
        MemoryPlacementGoal::AcceleratorVisible => MemoryPoolKind::UmaAccelVisible,
        MemoryPlacementGoal::DeviceLocal => MemoryPoolKind::GpuLocal,
        MemoryPlacementGoal::PeerFabric => MemoryPoolKind::PeerFabric,
        MemoryPlacementGoal::PartitionLocal => MemoryPoolKind::PartitionLocal,
        MemoryPlacementGoal::StagingIo => MemoryPoolKind::StagingIo,
    }
}

fn fallback_pools(goal: MemoryPlacementGoal) -> &'static [MemoryPoolKind] {
    match goal {
        MemoryPlacementGoal::SystemCoherent => &[],
        MemoryPlacementGoal::AcceleratorVisible => {
            &[MemoryPoolKind::HostPinned, MemoryPoolKind::SysCoherent]
        }
        MemoryPlacementGoal::DeviceLocal => &[MemoryPoolKind::UmaAccelVisible],
        MemoryPlacementGoal::PeerFabric => &[MemoryPoolKind::GpuLocal],
        MemoryPlacementGoal::PartitionLocal => &[MemoryPoolKind::GpuLocal],
        MemoryPlacementGoal::StagingIo => {
            &[MemoryPoolKind::HostPinned, MemoryPoolKind::SysCoherent]
        }
    }
}
