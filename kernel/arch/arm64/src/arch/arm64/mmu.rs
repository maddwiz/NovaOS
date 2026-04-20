use crate::bootinfo::NovaBootInfoV1;

pub const PAGE_SIZE: u64 = 4096;
const PAGE_MASK: u64 = PAGE_SIZE - 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PageTablePlan {
    pub kernel_base: u64,
    pub kernel_size: u64,
    pub user_base: u64,
    pub user_size: u64,
}

impl PageTablePlan {
    pub const fn empty() -> Self {
        Self {
            kernel_base: 0,
            kernel_size: 0,
            user_base: 0,
            user_size: 0,
        }
    }

    pub fn from_boot_info(boot_info: &NovaBootInfoV1) -> Self {
        let memory = boot_info.memory();
        Self {
            kernel_base: memory.kernel_window_base,
            kernel_size: memory.kernel_window_size,
            user_base: memory.user_window_base,
            user_size: memory.user_window_size,
        }
    }

    pub fn bootstrap_el0_mapping_plan(
        self,
        request: BootstrapEl0MappingRequest,
    ) -> BootstrapEl0MappingPlan {
        BootstrapEl0MappingPlan::from_page_table_plan(self, request)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BootstrapEl0MappingRequest {
    pub payload_source_base: u64,
    pub payload_size: u64,
    pub payload_entry_point: u64,
    pub context_source_base: u64,
    pub context_size: u64,
    pub user_stack_size: u64,
}

impl BootstrapEl0MappingRequest {
    pub const fn new(
        payload_source_base: u64,
        payload_size: u64,
        payload_entry_point: u64,
        context_source_base: u64,
        context_size: u64,
        user_stack_size: u64,
    ) -> Self {
        Self {
            payload_source_base,
            payload_size,
            payload_entry_point,
            context_source_base,
            context_size,
            user_stack_size,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BootstrapEl0MappingPlan {
    pub readiness: BootstrapEl0MappingReadiness,
    pub user_window_base: u64,
    pub user_window_size: u64,
    pub payload_source_base: u64,
    pub payload_source_size: u64,
    pub payload_source_map_base: u64,
    pub payload_source_map_size: u64,
    pub user_image_base: u64,
    pub user_image_size: u64,
    pub user_entry_point: u64,
    pub context_source_base: u64,
    pub user_context_base: u64,
    pub user_context_size: u64,
    pub user_stack_base: u64,
    pub user_stack_size: u64,
}

impl BootstrapEl0MappingPlan {
    fn from_page_table_plan(
        page_tables: PageTablePlan,
        request: BootstrapEl0MappingRequest,
    ) -> Self {
        let mut plan = Self {
            readiness: BootstrapEl0MappingReadiness::Ready,
            user_window_base: page_tables.user_base,
            user_window_size: page_tables.user_size,
            payload_source_base: request.payload_source_base,
            payload_source_size: request.payload_size,
            payload_source_map_base: 0,
            payload_source_map_size: 0,
            user_image_base: page_tables.user_base,
            user_image_size: 0,
            user_entry_point: 0,
            context_source_base: request.context_source_base,
            user_context_base: 0,
            user_context_size: 0,
            user_stack_base: 0,
            user_stack_size: 0,
        };

        let Some(user_window_end) = page_tables.user_base.checked_add(page_tables.user_size) else {
            plan.readiness = BootstrapEl0MappingReadiness::UserWindowAddressOverflow;
            return plan;
        };

        if page_tables.user_base == 0 || page_tables.user_size == 0 {
            plan.readiness = BootstrapEl0MappingReadiness::MissingUserWindow;
            return plan;
        }

        if !is_page_aligned(page_tables.user_base) || !is_page_aligned(page_tables.user_size) {
            plan.readiness = BootstrapEl0MappingReadiness::UnalignedUserWindow;
            return plan;
        }

        if request.payload_size == 0 {
            plan.readiness = BootstrapEl0MappingReadiness::EmptyPayload;
            return plan;
        }

        let Some(payload_end) = request
            .payload_source_base
            .checked_add(request.payload_size)
        else {
            plan.readiness = BootstrapEl0MappingReadiness::PayloadAddressOverflow;
            return plan;
        };

        if request.payload_entry_point < request.payload_source_base
            || request.payload_entry_point >= payload_end
        {
            plan.readiness = BootstrapEl0MappingReadiness::EntryOutsidePayload;
            return plan;
        }

        let Some((payload_source_map_base, payload_source_map_size)) =
            page_range(request.payload_source_base, request.payload_size)
        else {
            plan.readiness = BootstrapEl0MappingReadiness::PayloadAddressOverflow;
            return plan;
        };
        plan.payload_source_map_base = payload_source_map_base;
        plan.payload_source_map_size = payload_source_map_size;

        let Some(user_image_size) = align_up(request.payload_size) else {
            plan.readiness = BootstrapEl0MappingReadiness::PayloadAddressOverflow;
            return plan;
        };
        plan.user_image_size = user_image_size;

        let entry_offset = request.payload_entry_point - request.payload_source_base;
        let Some(user_entry_point) = plan.user_image_base.checked_add(entry_offset) else {
            plan.readiness = BootstrapEl0MappingReadiness::PayloadAddressOverflow;
            return plan;
        };
        plan.user_entry_point = user_entry_point;

        if request.context_source_base == 0 || request.context_size == 0 {
            plan.readiness = BootstrapEl0MappingReadiness::MissingContext;
            return plan;
        }

        if request
            .context_source_base
            .checked_add(request.context_size)
            .is_none()
        {
            plan.readiness = BootstrapEl0MappingReadiness::ContextAddressOverflow;
            return plan;
        }

        let Some(user_context_size) = align_up(request.context_size) else {
            plan.readiness = BootstrapEl0MappingReadiness::ContextAddressOverflow;
            return plan;
        };
        plan.user_context_size = user_context_size;

        let Some(user_context_base) = plan.user_image_base.checked_add(plan.user_image_size) else {
            plan.readiness = BootstrapEl0MappingReadiness::PayloadAddressOverflow;
            return plan;
        };
        plan.user_context_base = user_context_base;

        let Some(context_end) = user_context_base.checked_add(user_context_size) else {
            plan.readiness = BootstrapEl0MappingReadiness::ContextAddressOverflow;
            return plan;
        };

        if request.user_stack_size == 0 {
            plan.readiness = BootstrapEl0MappingReadiness::EmptyUserStack;
            return plan;
        }

        let Some(user_stack_size) = align_up(request.user_stack_size) else {
            plan.readiness = BootstrapEl0MappingReadiness::UserStackAddressOverflow;
            return plan;
        };
        plan.user_stack_size = user_stack_size;

        if user_stack_size > page_tables.user_size {
            plan.readiness = BootstrapEl0MappingReadiness::UserWindowTooSmall;
            return plan;
        }

        plan.user_stack_base = user_window_end - user_stack_size;

        if context_end > plan.user_stack_base {
            plan.readiness = BootstrapEl0MappingReadiness::UserWindowTooSmall;
            return plan;
        }

        plan
    }

    pub const fn ready(self) -> bool {
        matches!(self.readiness, BootstrapEl0MappingReadiness::Ready)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BootstrapEl0MappingReadiness {
    Ready,
    MissingUserWindow,
    UserWindowAddressOverflow,
    UnalignedUserWindow,
    EmptyPayload,
    PayloadAddressOverflow,
    EntryOutsidePayload,
    MissingContext,
    ContextAddressOverflow,
    EmptyUserStack,
    UserStackAddressOverflow,
    UserWindowTooSmall,
}

impl BootstrapEl0MappingReadiness {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::MissingUserWindow => "missing-user-window",
            Self::UserWindowAddressOverflow => "user-window-address-overflow",
            Self::UnalignedUserWindow => "unaligned-user-window",
            Self::EmptyPayload => "empty-payload",
            Self::PayloadAddressOverflow => "payload-address-overflow",
            Self::EntryOutsidePayload => "entry-outside-payload",
            Self::MissingContext => "missing-context",
            Self::ContextAddressOverflow => "context-address-overflow",
            Self::EmptyUserStack => "empty-user-stack",
            Self::UserStackAddressOverflow => "user-stack-address-overflow",
            Self::UserWindowTooSmall => "user-window-too-small",
        }
    }
}

const fn is_page_aligned(value: u64) -> bool {
    (value & PAGE_MASK) == 0
}

const fn align_down(value: u64) -> u64 {
    value & !PAGE_MASK
}

const fn align_up(value: u64) -> Option<u64> {
    if is_page_aligned(value) {
        Some(value)
    } else {
        match value.checked_add(PAGE_MASK) {
            Some(value) => Some(align_down(value)),
            None => None,
        }
    }
}

fn page_range(base: u64, size: u64) -> Option<(u64, u64)> {
    let end = base.checked_add(size)?;
    let map_base = align_down(base);
    let map_end = align_up(end)?;
    let map_size = map_end.checked_sub(map_base)?;
    Some((map_base, map_size))
}

#[cfg(test)]
mod tests {
    use super::{
        BootstrapEl0MappingReadiness, BootstrapEl0MappingRequest, PAGE_SIZE, PageTablePlan,
    };

    #[test]
    fn bootstrap_el0_mapping_plan_rebases_unaligned_payload_source_into_user_window() {
        let page_tables = PageTablePlan {
            kernel_base: 0,
            kernel_size: 0,
            user_base: 0x4000_0000,
            user_size: 0x20_000,
        };
        let request = BootstrapEl0MappingRequest::new(
            0x8020_0098,
            0x1234,
            0x8020_00A0,
            0x8100_0000,
            96,
            0x8000,
        );

        let plan = page_tables.bootstrap_el0_mapping_plan(request);

        assert!(plan.ready());
        assert_eq!(plan.readiness, BootstrapEl0MappingReadiness::Ready);
        assert_eq!(plan.payload_source_map_base, 0x8020_0000);
        assert_eq!(plan.payload_source_map_size, PAGE_SIZE * 2);
        assert_eq!(plan.user_image_base, 0x4000_0000);
        assert_eq!(plan.user_image_size, PAGE_SIZE * 2);
        assert_eq!(plan.user_entry_point, 0x4000_0008);
        assert_eq!(plan.user_context_base, 0x4000_2000);
        assert_eq!(plan.user_context_size, PAGE_SIZE);
        assert_eq!(plan.user_stack_base, 0x4001_8000);
        assert_eq!(plan.user_stack_size, 0x8000);
    }

    #[test]
    fn bootstrap_el0_mapping_plan_rejects_placeholder_bootinfo_user_window() {
        let page_tables = PageTablePlan {
            kernel_base: 0,
            kernel_size: 0,
            user_base: 0x2000,
            user_size: 3,
        };
        let request =
            BootstrapEl0MappingRequest::new(0x8020_0098, 4, 0x8020_0098, 0x8100_0000, 96, 0x4000);

        let plan = page_tables.bootstrap_el0_mapping_plan(request);

        assert_eq!(
            plan.readiness,
            BootstrapEl0MappingReadiness::UnalignedUserWindow
        );
    }

    #[test]
    fn bootstrap_el0_mapping_plan_rejects_entry_outside_payload() {
        let page_tables = PageTablePlan {
            kernel_base: 0,
            kernel_size: 0,
            user_base: 0x4000_0000,
            user_size: 0x20_000,
        };
        let request = BootstrapEl0MappingRequest::new(
            0x8020_0000,
            0x1000,
            0x8020_1000,
            0x8100_0000,
            96,
            0x4000,
        );

        let plan = page_tables.bootstrap_el0_mapping_plan(request);

        assert_eq!(
            plan.readiness,
            BootstrapEl0MappingReadiness::EntryOutsidePayload
        );
    }

    #[test]
    fn bootstrap_el0_mapping_plan_rejects_user_window_without_stack_room() {
        let page_tables = PageTablePlan {
            kernel_base: 0,
            kernel_size: 0,
            user_base: 0x4000_0000,
            user_size: 0x4000,
        };
        let request = BootstrapEl0MappingRequest::new(
            0x8020_0000,
            0x3000,
            0x8020_0000,
            0x8100_0000,
            96,
            0x1000,
        );

        let plan = page_tables.bootstrap_el0_mapping_plan(request);

        assert_eq!(
            plan.readiness,
            BootstrapEl0MappingReadiness::UserWindowTooSmall
        );
    }
}
