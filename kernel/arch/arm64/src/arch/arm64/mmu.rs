use crate::bootinfo::{NovaBootInfoV1, NovaBootInfoV2};

pub const PAGE_SIZE: u64 = 4096;
const PAGE_MASK: u64 = PAGE_SIZE - 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PageTablePlan {
    pub kernel_base: u64,
    pub kernel_size: u64,
    pub user_base: u64,
    pub user_size: u64,
    pub user_stack_size: u64,
}

impl PageTablePlan {
    pub const fn empty() -> Self {
        Self {
            kernel_base: 0,
            kernel_size: 0,
            user_base: 0,
            user_size: 0,
            user_stack_size: 0,
        }
    }

    pub fn from_boot_info(boot_info: &NovaBootInfoV1) -> Self {
        let memory = boot_info.memory();
        Self {
            kernel_base: memory.kernel_window_base,
            kernel_size: memory.kernel_window_size,
            user_base: memory.user_window_base,
            user_size: memory.user_window_size,
            user_stack_size: 0,
        }
    }

    pub fn from_boot_info_with_v2(
        boot_info: &NovaBootInfoV1,
        boot_info_v2: Option<&NovaBootInfoV2>,
    ) -> Self {
        let mut plan = Self::from_boot_info(boot_info);
        if let Some(user_window) = boot_info_v2
            .map(|boot_info_v2| boot_info_v2.bootstrap_user_window)
            .filter(|user_window| !user_window.is_empty() && user_window.is_valid())
        {
            plan.user_base = user_window.base;
            plan.user_size = user_window.len;
            plan.user_stack_size = user_window.stack_size;
        }
        plan
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

    pub fn page_table_plan(
        self,
        request: BootstrapEl0PageTableRequest,
    ) -> BootstrapEl0PageTablePlan {
        BootstrapEl0PageTablePlan::from_mapping_plan(self, request)
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Arm64MemoryAttr {
    NormalCached,
    DeviceNgNre,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Arm64AccessPerm {
    KernelReadWrite,
    UserReadOnly,
    UserReadExecute,
    UserReadWrite,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Arm64PageMapping {
    pub virt_base: u64,
    pub phys_base: u64,
    pub size: u64,
    pub attr: Arm64MemoryAttr,
    pub access: Arm64AccessPerm,
    pub execute_never: bool,
}

impl Arm64PageMapping {
    pub const fn empty() -> Self {
        Self {
            virt_base: 0,
            phys_base: 0,
            size: 0,
            attr: Arm64MemoryAttr::NormalCached,
            access: Arm64AccessPerm::KernelReadWrite,
            execute_never: true,
        }
    }

    pub const fn new(
        virt_base: u64,
        phys_base: u64,
        size: u64,
        attr: Arm64MemoryAttr,
        access: Arm64AccessPerm,
        execute_never: bool,
    ) -> Self {
        Self {
            virt_base,
            phys_base,
            size,
            attr,
            access,
            execute_never,
        }
    }

    pub const fn is_page_aligned(self) -> bool {
        is_page_aligned(self.virt_base)
            && is_page_aligned(self.phys_base)
            && is_page_aligned(self.size)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BootstrapEl0PageTableRequest {
    pub kernel_base: u64,
    pub kernel_size: u64,
    pub user_image_phys_base: u64,
    pub user_context_phys_base: u64,
    pub user_stack_phys_base: u64,
}

impl BootstrapEl0PageTableRequest {
    pub const fn new(
        kernel_base: u64,
        kernel_size: u64,
        user_image_phys_base: u64,
        user_context_phys_base: u64,
        user_stack_phys_base: u64,
    ) -> Self {
        Self {
            kernel_base,
            kernel_size,
            user_image_phys_base,
            user_context_phys_base,
            user_stack_phys_base,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BootstrapEl0PageTablePlan {
    pub readiness: BootstrapEl0PageTableReadiness,
    pub source_readiness: BootstrapEl0MappingReadiness,
    pub payload_copy_source_base: u64,
    pub payload_copy_source_size: u64,
    pub kernel_mapping: Arm64PageMapping,
    pub user_image_mapping: Arm64PageMapping,
    pub user_context_mapping: Arm64PageMapping,
    pub user_stack_mapping: Arm64PageMapping,
}

impl BootstrapEl0PageTablePlan {
    fn from_mapping_plan(
        mapping: BootstrapEl0MappingPlan,
        request: BootstrapEl0PageTableRequest,
    ) -> Self {
        let mut plan = Self {
            readiness: BootstrapEl0PageTableReadiness::Ready,
            source_readiness: mapping.readiness,
            payload_copy_source_base: mapping.payload_source_base,
            payload_copy_source_size: mapping.payload_source_size,
            kernel_mapping: Arm64PageMapping::empty(),
            user_image_mapping: Arm64PageMapping::empty(),
            user_context_mapping: Arm64PageMapping::empty(),
            user_stack_mapping: Arm64PageMapping::empty(),
        };

        if !mapping.ready() {
            plan.readiness = BootstrapEl0PageTableReadiness::MappingPlanNotReady;
            return plan;
        }

        if request.kernel_base == 0 || request.kernel_size == 0 {
            plan.readiness = BootstrapEl0PageTableReadiness::MissingKernelMapping;
            return plan;
        }

        let Some(kernel_size) = align_up(request.kernel_size) else {
            plan.readiness = BootstrapEl0PageTableReadiness::MappingAddressOverflow;
            return plan;
        };

        plan.kernel_mapping = Arm64PageMapping::new(
            request.kernel_base,
            request.kernel_base,
            kernel_size,
            Arm64MemoryAttr::NormalCached,
            Arm64AccessPerm::KernelReadWrite,
            false,
        );
        plan.user_image_mapping = Arm64PageMapping::new(
            mapping.user_image_base,
            request.user_image_phys_base,
            mapping.user_image_size,
            Arm64MemoryAttr::NormalCached,
            Arm64AccessPerm::UserReadExecute,
            false,
        );
        plan.user_context_mapping = Arm64PageMapping::new(
            mapping.user_context_base,
            request.user_context_phys_base,
            mapping.user_context_size,
            Arm64MemoryAttr::NormalCached,
            Arm64AccessPerm::UserReadOnly,
            true,
        );
        plan.user_stack_mapping = Arm64PageMapping::new(
            mapping.user_stack_base,
            request.user_stack_phys_base,
            mapping.user_stack_size,
            Arm64MemoryAttr::NormalCached,
            Arm64AccessPerm::UserReadWrite,
            true,
        );

        if !plan.kernel_mapping.is_page_aligned()
            || !plan.user_image_mapping.is_page_aligned()
            || !plan.user_context_mapping.is_page_aligned()
            || !plan.user_stack_mapping.is_page_aligned()
        {
            plan.readiness = BootstrapEl0PageTableReadiness::UnalignedMapping;
            return plan;
        }

        if mapping_end(plan.kernel_mapping).is_none()
            || mapping_end(plan.user_image_mapping).is_none()
            || mapping_end(plan.user_context_mapping).is_none()
            || mapping_end(plan.user_stack_mapping).is_none()
        {
            plan.readiness = BootstrapEl0PageTableReadiness::MappingAddressOverflow;
            return plan;
        }

        plan
    }

    pub const fn ready(self) -> bool {
        matches!(self.readiness, BootstrapEl0PageTableReadiness::Ready)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BootstrapEl0PageTableReadiness {
    Ready,
    MappingPlanNotReady,
    MissingKernelMapping,
    UnalignedMapping,
    MappingAddressOverflow,
}

impl BootstrapEl0PageTableReadiness {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::MappingPlanNotReady => "mapping-plan-not-ready",
            Self::MissingKernelMapping => "missing-kernel-mapping",
            Self::UnalignedMapping => "unaligned-mapping",
            Self::MappingAddressOverflow => "mapping-address-overflow",
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

fn mapping_end(mapping: Arm64PageMapping) -> Option<u64> {
    mapping.phys_base.checked_add(mapping.size)?;
    mapping.virt_base.checked_add(mapping.size)
}

#[cfg(test)]
mod tests {
    use super::{
        Arm64AccessPerm, Arm64MemoryAttr, BootstrapEl0MappingReadiness, BootstrapEl0MappingRequest,
        BootstrapEl0PageTableReadiness, BootstrapEl0PageTableRequest, PAGE_SIZE, PageTablePlan,
    };
    use crate::bootinfo::{NovaBootInfoV1, NovaBootInfoV2, NovaBootstrapUserWindowDescriptorV1};

    #[test]
    fn page_table_plan_prefers_valid_bootinfo_v2_bootstrap_user_window() {
        let mut boot_info = NovaBootInfoV1::new();
        boot_info.memory_map_ptr = 0x1000;
        boot_info.memory_map_entries = 4;
        boot_info.memory_map_desc_size = 48;
        boot_info.config_tables_ptr = 0x2000;
        boot_info.config_table_count = 3;
        let mut boot_info_v2 = NovaBootInfoV2::new();
        boot_info_v2.bootstrap_user_window = NovaBootstrapUserWindowDescriptorV1 {
            base: 0x4000_0000,
            len: 0x20_000,
            stack_size: 0x8000,
            page_size: NovaBootstrapUserWindowDescriptorV1::PAGE_SIZE_4K,
            flags: 0,
        };

        let plan = PageTablePlan::from_boot_info_with_v2(&boot_info, Some(&boot_info_v2));

        assert_eq!(plan.kernel_base, 0x1000);
        assert_eq!(plan.kernel_size, 4 * 48);
        assert_eq!(plan.user_base, 0x4000_0000);
        assert_eq!(plan.user_size, 0x20_000);
        assert_eq!(plan.user_stack_size, 0x8000);
    }

    #[test]
    fn page_table_plan_keeps_v1_placeholder_when_bootinfo_v2_window_is_empty() {
        let mut boot_info = NovaBootInfoV1::new();
        boot_info.config_tables_ptr = 0x2000;
        boot_info.config_table_count = 3;
        let boot_info_v2 = NovaBootInfoV2::new();

        let plan = PageTablePlan::from_boot_info_with_v2(&boot_info, Some(&boot_info_v2));

        assert_eq!(plan.user_base, 0x2000);
        assert_eq!(plan.user_size, 3);
        assert_eq!(plan.user_stack_size, 0);
    }

    #[test]
    fn bootstrap_el0_mapping_plan_rebases_unaligned_payload_source_into_user_window() {
        let page_tables = PageTablePlan {
            kernel_base: 0,
            kernel_size: 0,
            user_base: 0x4000_0000,
            user_size: 0x20_000,
            user_stack_size: 0,
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
            user_stack_size: 0,
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
            user_stack_size: 0,
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
            user_stack_size: 0,
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

    #[test]
    fn bootstrap_el0_page_table_plan_builds_copy_based_user_mappings() {
        let mapping = ready_bootstrap_el0_mapping_plan();
        let request = BootstrapEl0PageTableRequest::new(
            0x1000_0000,
            0x5000,
            0x9000_0000,
            0x9000_2000,
            0x9000_3000,
        );

        let plan = mapping.page_table_plan(request);

        assert!(plan.ready());
        assert_eq!(plan.readiness, BootstrapEl0PageTableReadiness::Ready);
        assert_eq!(plan.source_readiness, BootstrapEl0MappingReadiness::Ready);
        assert_eq!(plan.payload_copy_source_base, 0x8020_0098);
        assert_eq!(plan.payload_copy_source_size, 0x1234);
        assert_ne!(
            plan.user_image_mapping.phys_base,
            mapping.payload_source_map_base
        );
        assert_eq!(plan.kernel_mapping.virt_base, 0x1000_0000);
        assert_eq!(plan.kernel_mapping.phys_base, 0x1000_0000);
        assert_eq!(plan.kernel_mapping.size, 0x5000);
        assert_eq!(plan.kernel_mapping.access, Arm64AccessPerm::KernelReadWrite);
        assert!(!plan.kernel_mapping.execute_never);
        assert_eq!(plan.user_image_mapping.virt_base, mapping.user_image_base);
        assert_eq!(plan.user_image_mapping.phys_base, 0x9000_0000);
        assert_eq!(plan.user_image_mapping.size, mapping.user_image_size);
        assert_eq!(
            plan.user_image_mapping.access,
            Arm64AccessPerm::UserReadExecute
        );
        assert!(!plan.user_image_mapping.execute_never);
        assert_eq!(
            plan.user_context_mapping.virt_base,
            mapping.user_context_base
        );
        assert_eq!(plan.user_context_mapping.phys_base, 0x9000_2000);
        assert_eq!(
            plan.user_context_mapping.access,
            Arm64AccessPerm::UserReadOnly
        );
        assert!(plan.user_context_mapping.execute_never);
        assert_eq!(plan.user_stack_mapping.virt_base, mapping.user_stack_base);
        assert_eq!(plan.user_stack_mapping.phys_base, 0x9000_3000);
        assert_eq!(
            plan.user_stack_mapping.access,
            Arm64AccessPerm::UserReadWrite
        );
        assert!(plan.user_stack_mapping.execute_never);
        assert_eq!(plan.user_stack_mapping.attr, Arm64MemoryAttr::NormalCached);
        assert!(plan.kernel_mapping.is_page_aligned());
        assert!(plan.user_image_mapping.is_page_aligned());
        assert!(plan.user_context_mapping.is_page_aligned());
        assert!(plan.user_stack_mapping.is_page_aligned());
    }

    #[test]
    fn bootstrap_el0_page_table_plan_refuses_unready_mapping_plan() {
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
        let request = BootstrapEl0PageTableRequest::new(
            0x1000_0000,
            0x5000,
            0x9000_0000,
            0x9000_1000,
            0x9000_2000,
        );

        let plan = mapping.page_table_plan(request);

        assert_eq!(
            plan.readiness,
            BootstrapEl0PageTableReadiness::MappingPlanNotReady
        );
        assert_eq!(
            plan.source_readiness,
            BootstrapEl0MappingReadiness::UnalignedUserWindow
        );
    }

    #[test]
    fn bootstrap_el0_page_table_plan_rejects_unaligned_physical_mapping() {
        let mapping = ready_bootstrap_el0_mapping_plan();
        let request = BootstrapEl0PageTableRequest::new(
            0x1000_0000,
            0x5000,
            0x9000_0001,
            0x9000_2000,
            0x9000_3000,
        );

        let plan = mapping.page_table_plan(request);

        assert_eq!(
            plan.readiness,
            BootstrapEl0PageTableReadiness::UnalignedMapping
        );
    }

    fn ready_bootstrap_el0_mapping_plan() -> super::BootstrapEl0MappingPlan {
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
