use crate::bootinfo::NovaBootInfoV1;

use super::mmu::{
    BootstrapEl0MappingPlan, BootstrapEl0MappingReadiness, BootstrapEl0PageTableRequest, PAGE_SIZE,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameAllocatorPlan {
    pub usable_base: u64,
    pub usable_limit: u64,
    pub reserved_bytes: u64,
}

impl FrameAllocatorPlan {
    pub const fn empty() -> Self {
        Self {
            usable_base: 0,
            usable_limit: 0,
            reserved_bytes: 0,
        }
    }

    pub fn from_boot_info(boot_info: &NovaBootInfoV1) -> Self {
        let memory = boot_info.memory();
        Self {
            usable_base: memory.usable_base,
            usable_limit: memory.usable_limit,
            reserved_bytes: memory.reserved_bytes,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BootstrapEl0BackingFrameRequest {
    pub arena_base: u64,
    pub arena_size: u64,
}

impl BootstrapEl0BackingFrameRequest {
    pub const fn new(arena_base: u64, arena_size: u64) -> Self {
        Self {
            arena_base,
            arena_size,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BootstrapEl0BackingFramePlan {
    pub readiness: BootstrapEl0BackingFrameReadiness,
    pub source_readiness: BootstrapEl0MappingReadiness,
    pub arena_base: u64,
    pub arena_size: u64,
    pub image_phys_base: u64,
    pub image_size: u64,
    pub context_phys_base: u64,
    pub context_size: u64,
    pub stack_phys_base: u64,
    pub stack_size: u64,
    pub total_size: u64,
}

impl BootstrapEl0BackingFramePlan {
    pub fn from_mapping_plan(
        mapping: BootstrapEl0MappingPlan,
        request: BootstrapEl0BackingFrameRequest,
    ) -> Self {
        let mut plan = Self {
            readiness: BootstrapEl0BackingFrameReadiness::Ready,
            source_readiness: mapping.readiness,
            arena_base: request.arena_base,
            arena_size: request.arena_size,
            image_phys_base: 0,
            image_size: mapping.user_image_size,
            context_phys_base: 0,
            context_size: mapping.user_context_size,
            stack_phys_base: 0,
            stack_size: mapping.user_stack_size,
            total_size: 0,
        };

        if !mapping.ready() {
            plan.readiness = BootstrapEl0BackingFrameReadiness::MappingPlanNotReady;
            return plan;
        }

        if request.arena_base == 0 || request.arena_size == 0 {
            plan.readiness = BootstrapEl0BackingFrameReadiness::MissingArena;
            return plan;
        }

        if !is_page_aligned(request.arena_base) || !is_page_aligned(request.arena_size) {
            plan.readiness = BootstrapEl0BackingFrameReadiness::UnalignedArena;
            return plan;
        }

        let Some(arena_end) = request.arena_base.checked_add(request.arena_size) else {
            plan.readiness = BootstrapEl0BackingFrameReadiness::ArenaAddressOverflow;
            return plan;
        };

        let Some(context_phys_base) = request.arena_base.checked_add(mapping.user_image_size)
        else {
            plan.readiness = BootstrapEl0BackingFrameReadiness::FrameAddressOverflow;
            return plan;
        };
        let Some(stack_phys_base) = context_phys_base.checked_add(mapping.user_context_size) else {
            plan.readiness = BootstrapEl0BackingFrameReadiness::FrameAddressOverflow;
            return plan;
        };
        let Some(frame_end) = stack_phys_base.checked_add(mapping.user_stack_size) else {
            plan.readiness = BootstrapEl0BackingFrameReadiness::FrameAddressOverflow;
            return plan;
        };

        if frame_end > arena_end {
            plan.readiness = BootstrapEl0BackingFrameReadiness::ArenaTooSmall;
            return plan;
        }

        plan.image_phys_base = request.arena_base;
        plan.context_phys_base = context_phys_base;
        plan.stack_phys_base = stack_phys_base;
        plan.total_size = frame_end - request.arena_base;
        plan
    }

    pub const fn ready(self) -> bool {
        matches!(self.readiness, BootstrapEl0BackingFrameReadiness::Ready)
    }

    pub const fn page_table_request(
        self,
        kernel_base: u64,
        kernel_size: u64,
    ) -> BootstrapEl0PageTableRequest {
        BootstrapEl0PageTableRequest::new(
            kernel_base,
            kernel_size,
            self.image_phys_base,
            self.context_phys_base,
            self.stack_phys_base,
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BootstrapEl0BackingFrameReadiness {
    Ready,
    MappingPlanNotReady,
    MissingArena,
    UnalignedArena,
    ArenaAddressOverflow,
    FrameAddressOverflow,
    ArenaTooSmall,
}

impl BootstrapEl0BackingFrameReadiness {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::MappingPlanNotReady => "mapping-plan-not-ready",
            Self::MissingArena => "missing-arena",
            Self::UnalignedArena => "unaligned-arena",
            Self::ArenaAddressOverflow => "arena-address-overflow",
            Self::FrameAddressOverflow => "frame-address-overflow",
            Self::ArenaTooSmall => "arena-too-small",
        }
    }
}

const fn is_page_aligned(value: u64) -> bool {
    (value & (PAGE_SIZE - 1)) == 0
}

#[cfg(test)]
mod tests {
    use super::{
        BootstrapEl0BackingFramePlan, BootstrapEl0BackingFrameReadiness,
        BootstrapEl0BackingFrameRequest,
    };
    use crate::arch::arm64::mmu::{
        BootstrapEl0MappingReadiness, BootstrapEl0MappingRequest, PageTablePlan,
    };

    #[test]
    fn bootstrap_el0_backing_frame_plan_carves_image_context_and_stack_frames() {
        let mapping = ready_mapping_plan();
        let backing = BootstrapEl0BackingFramePlan::from_mapping_plan(
            mapping,
            BootstrapEl0BackingFrameRequest::new(0x9000_0000, 0x20_000),
        );

        assert!(backing.ready());
        assert_eq!(backing.readiness, BootstrapEl0BackingFrameReadiness::Ready);
        assert_eq!(
            backing.source_readiness,
            BootstrapEl0MappingReadiness::Ready
        );
        assert_eq!(backing.image_phys_base, 0x9000_0000);
        assert_eq!(backing.image_size, 0x2000);
        assert_eq!(backing.context_phys_base, 0x9000_2000);
        assert_eq!(backing.context_size, 0x1000);
        assert_eq!(backing.stack_phys_base, 0x9000_3000);
        assert_eq!(backing.stack_size, 0x8000);
        assert_eq!(backing.total_size, 0xB000);

        let page_tables = mapping.page_table_plan(backing.page_table_request(0x1000_0000, 0x5000));
        assert!(page_tables.ready());
        assert_eq!(
            page_tables.user_image_mapping.phys_base,
            backing.image_phys_base
        );
        assert_eq!(
            page_tables.user_context_mapping.phys_base,
            backing.context_phys_base
        );
        assert_eq!(
            page_tables.user_stack_mapping.phys_base,
            backing.stack_phys_base
        );
    }

    #[test]
    fn bootstrap_el0_backing_frame_plan_refuses_unready_mapping_plan() {
        let page_tables = PageTablePlan {
            kernel_base: 0,
            kernel_size: 0,
            user_base: 0x2000,
            user_size: 3,
            user_stack_size: 0,
        };
        let mapping = page_tables.bootstrap_el0_mapping_plan(BootstrapEl0MappingRequest::new(
            0x8020_0098,
            4,
            0x8020_0098,
            0x8100_0000,
            96,
            0x4000,
        ));

        let backing = BootstrapEl0BackingFramePlan::from_mapping_plan(
            mapping,
            BootstrapEl0BackingFrameRequest::new(0x9000_0000, 0x20_000),
        );

        assert_eq!(
            backing.readiness,
            BootstrapEl0BackingFrameReadiness::MappingPlanNotReady
        );
        assert_eq!(
            backing.source_readiness,
            BootstrapEl0MappingReadiness::UnalignedUserWindow
        );
    }

    #[test]
    fn bootstrap_el0_backing_frame_plan_rejects_unaligned_arena() {
        let backing = BootstrapEl0BackingFramePlan::from_mapping_plan(
            ready_mapping_plan(),
            BootstrapEl0BackingFrameRequest::new(0x9000_0001, 0x20_000),
        );

        assert_eq!(
            backing.readiness,
            BootstrapEl0BackingFrameReadiness::UnalignedArena
        );
    }

    #[test]
    fn bootstrap_el0_backing_frame_plan_rejects_small_arena() {
        let backing = BootstrapEl0BackingFramePlan::from_mapping_plan(
            ready_mapping_plan(),
            BootstrapEl0BackingFrameRequest::new(0x9000_0000, 0xA000),
        );

        assert_eq!(
            backing.readiness,
            BootstrapEl0BackingFrameReadiness::ArenaTooSmall
        );
    }

    fn ready_mapping_plan() -> crate::arch::arm64::mmu::BootstrapEl0MappingPlan {
        let page_tables = PageTablePlan {
            kernel_base: 0,
            kernel_size: 0,
            user_base: 0x4000_0000,
            user_size: 0x20_000,
            user_stack_size: 0,
        };
        page_tables.bootstrap_el0_mapping_plan(BootstrapEl0MappingRequest::new(
            0x8020_0098,
            0x1234,
            0x8020_00A0,
            0x8100_0000,
            96,
            0x8000,
        ))
    }
}
