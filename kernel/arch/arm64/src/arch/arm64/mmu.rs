use crate::bootinfo::{NovaBootInfoV1, NovaBootInfoV2};

pub const PAGE_SIZE: u64 = 4096;
const PAGE_MASK: u64 = PAGE_SIZE - 1;
const ARM64_TABLE_ENTRY_COUNT: usize = 512;
const ARM64_TABLE_ENTRY_SIZE: u64 = 8;
const ROOT_TABLE_INDEX: usize = 0;
const L1_TABLE_INDEX: usize = 1;
const KERNEL_L2_TABLE_INDEX: usize = 2;
const KERNEL_L3_TABLE_INDEX: usize = 3;
const USER_L2_TABLE_INDEX: usize = 4;
const USER_L3_TABLE_INDEX: usize = 5;
pub const BOOTSTRAP_EL0_TRANSLATION_TABLE_COUNT: u64 = 6;
pub const BOOTSTRAP_EL0_TRANSLATION_TABLE_BYTES: u64 =
    BOOTSTRAP_EL0_TRANSLATION_TABLE_COUNT * PAGE_SIZE;
const BOOTSTRAP_EL0_TRANSLATION_TABLE_ENTRIES: usize =
    (BOOTSTRAP_EL0_TRANSLATION_TABLE_BYTES / ARM64_TABLE_ENTRY_SIZE) as usize;

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
    pub context_source_size: u64,
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
            context_source_size: request.context_size,
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
    pub page_table_phys_base: u64,
    pub page_table_size: u64,
}

impl BootstrapEl0PageTableRequest {
    pub const fn new(
        kernel_base: u64,
        kernel_size: u64,
        user_image_phys_base: u64,
        user_context_phys_base: u64,
        user_stack_phys_base: u64,
        page_table_phys_base: u64,
        page_table_size: u64,
    ) -> Self {
        Self {
            kernel_base,
            kernel_size,
            user_image_phys_base,
            user_context_phys_base,
            user_stack_phys_base,
            page_table_phys_base,
            page_table_size,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BootstrapEl0PageTablePlan {
    pub readiness: BootstrapEl0PageTableReadiness,
    pub source_readiness: BootstrapEl0MappingReadiness,
    pub payload_copy_source_base: u64,
    pub payload_copy_source_size: u64,
    pub context_copy_source_base: u64,
    pub context_copy_source_size: u64,
    pub kernel_mapping: Arm64PageMapping,
    pub user_image_mapping: Arm64PageMapping,
    pub user_context_mapping: Arm64PageMapping,
    pub user_stack_mapping: Arm64PageMapping,
    pub page_table_phys_base: u64,
    pub page_table_size: u64,
    pub page_table_bytes: u64,
    pub root_table_phys_base: u64,
    pub l1_table_phys_base: u64,
    pub kernel_l2_table_phys_base: u64,
    pub kernel_l3_table_phys_base: u64,
    pub user_l2_table_phys_base: u64,
    pub user_l3_table_phys_base: u64,
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
            context_copy_source_base: mapping.context_source_base,
            context_copy_source_size: mapping.context_source_size,
            kernel_mapping: Arm64PageMapping::empty(),
            user_image_mapping: Arm64PageMapping::empty(),
            user_context_mapping: Arm64PageMapping::empty(),
            user_stack_mapping: Arm64PageMapping::empty(),
            page_table_phys_base: request.page_table_phys_base,
            page_table_size: request.page_table_size,
            page_table_bytes: 0,
            root_table_phys_base: 0,
            l1_table_phys_base: 0,
            kernel_l2_table_phys_base: 0,
            kernel_l3_table_phys_base: 0,
            user_l2_table_phys_base: 0,
            user_l3_table_phys_base: 0,
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

        if request.page_table_phys_base == 0 || request.page_table_size == 0 {
            plan.readiness = BootstrapEl0PageTableReadiness::MissingPageTableArena;
            return plan;
        }

        if !is_page_aligned(request.page_table_phys_base)
            || !is_page_aligned(request.page_table_size)
        {
            plan.readiness = BootstrapEl0PageTableReadiness::UnalignedPageTableArena;
            return plan;
        }

        let Some(page_table_arena_end) = request
            .page_table_phys_base
            .checked_add(request.page_table_size)
        else {
            plan.readiness = BootstrapEl0PageTableReadiness::MappingAddressOverflow;
            return plan;
        };

        let Some(page_table_end) = request
            .page_table_phys_base
            .checked_add(BOOTSTRAP_EL0_TRANSLATION_TABLE_BYTES)
        else {
            plan.readiness = BootstrapEl0PageTableReadiness::MappingAddressOverflow;
            return plan;
        };

        if page_table_end > page_table_arena_end {
            plan.readiness = BootstrapEl0PageTableReadiness::PageTableArenaTooSmall;
            return plan;
        }

        if BootstrapEl0TranslationTableLayout::from_plan(plan).is_none() {
            plan.readiness = BootstrapEl0PageTableReadiness::UnsupportedVirtualAddressLayout;
            return plan;
        }

        plan.page_table_bytes = BOOTSTRAP_EL0_TRANSLATION_TABLE_BYTES;
        plan.root_table_phys_base = request.page_table_phys_base;
        plan.l1_table_phys_base = table_phys_base(request.page_table_phys_base, L1_TABLE_INDEX);
        plan.kernel_l2_table_phys_base =
            table_phys_base(request.page_table_phys_base, KERNEL_L2_TABLE_INDEX);
        plan.kernel_l3_table_phys_base =
            table_phys_base(request.page_table_phys_base, KERNEL_L3_TABLE_INDEX);
        plan.user_l2_table_phys_base =
            table_phys_base(request.page_table_phys_base, USER_L2_TABLE_INDEX);
        plan.user_l3_table_phys_base =
            table_phys_base(request.page_table_phys_base, USER_L3_TABLE_INDEX);

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
    MissingPageTableArena,
    UnalignedMapping,
    UnalignedPageTableArena,
    PageTableArenaTooSmall,
    UnsupportedVirtualAddressLayout,
    MappingAddressOverflow,
}

impl BootstrapEl0PageTableReadiness {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::MappingPlanNotReady => "mapping-plan-not-ready",
            Self::MissingKernelMapping => "missing-kernel-mapping",
            Self::MissingPageTableArena => "missing-page-table-arena",
            Self::UnalignedMapping => "unaligned-mapping",
            Self::UnalignedPageTableArena => "unaligned-page-table-arena",
            Self::PageTableArenaTooSmall => "page-table-arena-too-small",
            Self::UnsupportedVirtualAddressLayout => "unsupported-virtual-address-layout",
            Self::MappingAddressOverflow => "mapping-address-overflow",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BootstrapEl0PageTableConstruction {
    pub readiness: BootstrapEl0PageTableConstructionReadiness,
    pub source_readiness: BootstrapEl0PageTableReadiness,
    pub root_table_phys_base: u64,
    pub page_table_bytes: u64,
    pub mapped_pages: u64,
}

impl BootstrapEl0PageTableConstruction {
    const fn from_plan(
        plan: BootstrapEl0PageTablePlan,
        readiness: BootstrapEl0PageTableConstructionReadiness,
    ) -> Self {
        Self {
            readiness,
            source_readiness: plan.readiness,
            root_table_phys_base: plan.root_table_phys_base,
            page_table_bytes: plan.page_table_bytes,
            mapped_pages: 0,
        }
    }

    pub const fn ready(self) -> bool {
        matches!(
            self.readiness,
            BootstrapEl0PageTableConstructionReadiness::Ready
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BootstrapEl0PageTableConstructionReadiness {
    Ready,
    PageTablePlanNotReady,
    OutputBufferTooSmall,
    UnsupportedVirtualAddressLayout,
    MappingAddressOverflow,
}

impl BootstrapEl0PageTableConstructionReadiness {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::PageTablePlanNotReady => "page-table-plan-not-ready",
            Self::OutputBufferTooSmall => "output-buffer-too-small",
            Self::UnsupportedVirtualAddressLayout => "unsupported-virtual-address-layout",
            Self::MappingAddressOverflow => "mapping-address-overflow",
        }
    }
}

pub fn construct_bootstrap_el0_page_tables(
    plan: BootstrapEl0PageTablePlan,
    entries: &mut [u64],
) -> BootstrapEl0PageTableConstruction {
    if !plan.ready() {
        return BootstrapEl0PageTableConstruction::from_plan(
            plan,
            BootstrapEl0PageTableConstructionReadiness::PageTablePlanNotReady,
        );
    }

    if entries.len() < BOOTSTRAP_EL0_TRANSLATION_TABLE_ENTRIES {
        return BootstrapEl0PageTableConstruction::from_plan(
            plan,
            BootstrapEl0PageTableConstructionReadiness::OutputBufferTooSmall,
        );
    }

    let Some(layout) = BootstrapEl0TranslationTableLayout::from_plan(plan) else {
        return BootstrapEl0PageTableConstruction::from_plan(
            plan,
            BootstrapEl0PageTableConstructionReadiness::UnsupportedVirtualAddressLayout,
        );
    };

    for entry in entries
        .iter_mut()
        .take(BOOTSTRAP_EL0_TRANSLATION_TABLE_ENTRIES)
    {
        *entry = 0;
    }

    write_table_entry(
        entries,
        ROOT_TABLE_INDEX,
        layout.l0_index,
        table_descriptor(plan.l1_table_phys_base),
    );
    write_table_entry(
        entries,
        L1_TABLE_INDEX,
        layout.kernel_l1_index,
        table_descriptor(plan.kernel_l2_table_phys_base),
    );
    write_table_entry(
        entries,
        L1_TABLE_INDEX,
        layout.user_l1_index,
        table_descriptor(plan.user_l2_table_phys_base),
    );
    write_table_entry(
        entries,
        KERNEL_L2_TABLE_INDEX,
        layout.kernel_l2_index,
        table_descriptor(plan.kernel_l3_table_phys_base),
    );
    write_table_entry(
        entries,
        USER_L2_TABLE_INDEX,
        layout.user_l2_index,
        table_descriptor(plan.user_l3_table_phys_base),
    );

    let Some(kernel_pages) = write_page_mapping(
        entries,
        KERNEL_L3_TABLE_INDEX,
        plan.kernel_mapping,
        layout.kernel_l3_start,
    ) else {
        return BootstrapEl0PageTableConstruction::from_plan(
            plan,
            BootstrapEl0PageTableConstructionReadiness::MappingAddressOverflow,
        );
    };
    let Some(user_image_pages) = write_page_mapping(
        entries,
        USER_L3_TABLE_INDEX,
        plan.user_image_mapping,
        layout.user_image_l3_start,
    ) else {
        return BootstrapEl0PageTableConstruction::from_plan(
            plan,
            BootstrapEl0PageTableConstructionReadiness::MappingAddressOverflow,
        );
    };
    let Some(user_context_pages) = write_page_mapping(
        entries,
        USER_L3_TABLE_INDEX,
        plan.user_context_mapping,
        layout.user_context_l3_start,
    ) else {
        return BootstrapEl0PageTableConstruction::from_plan(
            plan,
            BootstrapEl0PageTableConstructionReadiness::MappingAddressOverflow,
        );
    };
    let Some(user_stack_pages) = write_page_mapping(
        entries,
        USER_L3_TABLE_INDEX,
        plan.user_stack_mapping,
        layout.user_stack_l3_start,
    ) else {
        return BootstrapEl0PageTableConstruction::from_plan(
            plan,
            BootstrapEl0PageTableConstructionReadiness::MappingAddressOverflow,
        );
    };

    let mut construction = BootstrapEl0PageTableConstruction::from_plan(
        plan,
        BootstrapEl0PageTableConstructionReadiness::Ready,
    );
    construction.mapped_pages =
        kernel_pages + user_image_pages + user_context_pages + user_stack_pages;
    construction
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BootstrapEl0BackingFramePopulation {
    pub readiness: BootstrapEl0BackingFramePopulationReadiness,
    pub source_readiness: BootstrapEl0PageTableReadiness,
    pub payload_bytes: u64,
    pub context_bytes: u64,
    pub zeroed_bytes: u64,
}

impl BootstrapEl0BackingFramePopulation {
    const fn from_plan(
        plan: BootstrapEl0PageTablePlan,
        readiness: BootstrapEl0BackingFramePopulationReadiness,
    ) -> Self {
        Self {
            readiness,
            source_readiness: plan.readiness,
            payload_bytes: 0,
            context_bytes: 0,
            zeroed_bytes: 0,
        }
    }

    pub const fn ready(self) -> bool {
        matches!(
            self.readiness,
            BootstrapEl0BackingFramePopulationReadiness::Ready
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BootstrapEl0BackingFramePopulationReadiness {
    Ready,
    PageTablePlanNotReady,
    LengthOverflow,
    PayloadSourceTooSmall,
    ContextSourceTooSmall,
    ImageFrameTooSmall,
    ContextFrameTooSmall,
    StackFrameTooSmall,
}

impl BootstrapEl0BackingFramePopulationReadiness {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::PageTablePlanNotReady => "page-table-plan-not-ready",
            Self::LengthOverflow => "length-overflow",
            Self::PayloadSourceTooSmall => "payload-source-too-small",
            Self::ContextSourceTooSmall => "context-source-too-small",
            Self::ImageFrameTooSmall => "image-frame-too-small",
            Self::ContextFrameTooSmall => "context-frame-too-small",
            Self::StackFrameTooSmall => "stack-frame-too-small",
        }
    }
}

pub fn populate_bootstrap_el0_backing_frames(
    plan: BootstrapEl0PageTablePlan,
    payload_source: &[u8],
    context_source: &[u8],
    image_frame: &mut [u8],
    context_frame: &mut [u8],
    stack_frame: &mut [u8],
) -> BootstrapEl0BackingFramePopulation {
    if !plan.ready() {
        return BootstrapEl0BackingFramePopulation::from_plan(
            plan,
            BootstrapEl0BackingFramePopulationReadiness::PageTablePlanNotReady,
        );
    }

    let Some(payload_len) = u64_to_usize(plan.payload_copy_source_size) else {
        return BootstrapEl0BackingFramePopulation::from_plan(
            plan,
            BootstrapEl0BackingFramePopulationReadiness::LengthOverflow,
        );
    };
    let Some(context_len) = u64_to_usize(plan.context_copy_source_size) else {
        return BootstrapEl0BackingFramePopulation::from_plan(
            plan,
            BootstrapEl0BackingFramePopulationReadiness::LengthOverflow,
        );
    };
    let Some(image_len) = u64_to_usize(plan.user_image_mapping.size) else {
        return BootstrapEl0BackingFramePopulation::from_plan(
            plan,
            BootstrapEl0BackingFramePopulationReadiness::LengthOverflow,
        );
    };
    let Some(context_frame_len) = u64_to_usize(plan.user_context_mapping.size) else {
        return BootstrapEl0BackingFramePopulation::from_plan(
            plan,
            BootstrapEl0BackingFramePopulationReadiness::LengthOverflow,
        );
    };
    let Some(stack_len) = u64_to_usize(plan.user_stack_mapping.size) else {
        return BootstrapEl0BackingFramePopulation::from_plan(
            plan,
            BootstrapEl0BackingFramePopulationReadiness::LengthOverflow,
        );
    };

    if payload_source.len() < payload_len {
        return BootstrapEl0BackingFramePopulation::from_plan(
            plan,
            BootstrapEl0BackingFramePopulationReadiness::PayloadSourceTooSmall,
        );
    }
    if context_source.len() < context_len {
        return BootstrapEl0BackingFramePopulation::from_plan(
            plan,
            BootstrapEl0BackingFramePopulationReadiness::ContextSourceTooSmall,
        );
    }
    if image_frame.len() < image_len || payload_len > image_len {
        return BootstrapEl0BackingFramePopulation::from_plan(
            plan,
            BootstrapEl0BackingFramePopulationReadiness::ImageFrameTooSmall,
        );
    }
    if context_frame.len() < context_frame_len || context_len > context_frame_len {
        return BootstrapEl0BackingFramePopulation::from_plan(
            plan,
            BootstrapEl0BackingFramePopulationReadiness::ContextFrameTooSmall,
        );
    }
    if stack_frame.len() < stack_len {
        return BootstrapEl0BackingFramePopulation::from_plan(
            plan,
            BootstrapEl0BackingFramePopulationReadiness::StackFrameTooSmall,
        );
    }

    image_frame[..image_len].fill(0);
    image_frame[..payload_len].copy_from_slice(&payload_source[..payload_len]);
    context_frame[..context_frame_len].fill(0);
    context_frame[..context_len].copy_from_slice(&context_source[..context_len]);
    stack_frame[..stack_len].fill(0);

    let mut population = BootstrapEl0BackingFramePopulation::from_plan(
        plan,
        BootstrapEl0BackingFramePopulationReadiness::Ready,
    );
    population.payload_bytes = plan.payload_copy_source_size;
    population.context_bytes = plan.context_copy_source_size;
    population.zeroed_bytes = plan
        .user_image_mapping
        .size
        .saturating_sub(plan.payload_copy_source_size)
        .saturating_add(
            plan.user_context_mapping
                .size
                .saturating_sub(plan.context_copy_source_size),
        )
        .saturating_add(plan.user_stack_mapping.size);
    population
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BootstrapEl0TranslationTableLayout {
    l0_index: usize,
    kernel_l1_index: usize,
    kernel_l2_index: usize,
    kernel_l3_start: usize,
    user_l1_index: usize,
    user_l2_index: usize,
    user_image_l3_start: usize,
    user_context_l3_start: usize,
    user_stack_l3_start: usize,
}

impl BootstrapEl0TranslationTableLayout {
    fn from_plan(plan: BootstrapEl0PageTablePlan) -> Option<Self> {
        let kernel = MappingSpan::from_mapping(plan.kernel_mapping)?;
        let user_image = MappingSpan::from_mapping(plan.user_image_mapping)?;
        let user_context = MappingSpan::from_mapping(plan.user_context_mapping)?;
        let user_stack = MappingSpan::from_mapping(plan.user_stack_mapping)?;

        if kernel.l0_index != user_image.l0_index
            || user_image.l0_index != user_context.l0_index
            || user_context.l0_index != user_stack.l0_index
        {
            return None;
        }

        if user_image.l1_index != user_context.l1_index
            || user_context.l1_index != user_stack.l1_index
            || user_image.l2_index != user_context.l2_index
            || user_context.l2_index != user_stack.l2_index
        {
            return None;
        }

        if kernel.l1_index == user_image.l1_index {
            return None;
        }

        Some(Self {
            l0_index: kernel.l0_index,
            kernel_l1_index: kernel.l1_index,
            kernel_l2_index: kernel.l2_index,
            kernel_l3_start: kernel.l3_start,
            user_l1_index: user_image.l1_index,
            user_l2_index: user_image.l2_index,
            user_image_l3_start: user_image.l3_start,
            user_context_l3_start: user_context.l3_start,
            user_stack_l3_start: user_stack.l3_start,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MappingSpan {
    l0_index: usize,
    l1_index: usize,
    l2_index: usize,
    l3_start: usize,
    page_count: u64,
}

impl MappingSpan {
    fn from_mapping(mapping: Arm64PageMapping) -> Option<Self> {
        if !mapping.is_page_aligned() || mapping.size == 0 {
            return None;
        }

        let end = mapping
            .virt_base
            .checked_add(mapping.size)?
            .checked_sub(1)?;
        if l0_index(mapping.virt_base) != l0_index(end)
            || l1_index(mapping.virt_base) != l1_index(end)
            || l2_index(mapping.virt_base) != l2_index(end)
        {
            return None;
        }

        let l3_start = l3_index(mapping.virt_base);
        let page_count = mapping.size / PAGE_SIZE;
        if l3_start as u64 + page_count > ARM64_TABLE_ENTRY_COUNT as u64 {
            return None;
        }

        Some(Self {
            l0_index: l0_index(mapping.virt_base),
            l1_index: l1_index(mapping.virt_base),
            l2_index: l2_index(mapping.virt_base),
            l3_start,
            page_count,
        })
    }
}

fn write_page_mapping(
    entries: &mut [u64],
    table_index: usize,
    mapping: Arm64PageMapping,
    l3_start: usize,
) -> Option<u64> {
    let span = MappingSpan::from_mapping(mapping)?;
    let mut mapped_pages = 0;
    while mapped_pages < span.page_count {
        let phys_base = mapping
            .phys_base
            .checked_add(mapped_pages.checked_mul(PAGE_SIZE)?)?;
        write_table_entry(
            entries,
            table_index,
            l3_start + mapped_pages as usize,
            page_descriptor(mapping, phys_base),
        );
        mapped_pages += 1;
    }
    Some(mapped_pages)
}

fn write_table_entry(entries: &mut [u64], table_index: usize, entry_index: usize, value: u64) {
    entries[table_entry_offset(table_index, entry_index)] = value;
}

const fn table_entry_offset(table_index: usize, entry_index: usize) -> usize {
    table_index * ARM64_TABLE_ENTRY_COUNT + entry_index
}

const fn table_phys_base(page_table_phys_base: u64, table_index: usize) -> u64 {
    page_table_phys_base + (table_index as u64 * PAGE_SIZE)
}

const fn l0_index(virt_base: u64) -> usize {
    ((virt_base >> 39) & 0x1ff) as usize
}

const fn l1_index(virt_base: u64) -> usize {
    ((virt_base >> 30) & 0x1ff) as usize
}

const fn l2_index(virt_base: u64) -> usize {
    ((virt_base >> 21) & 0x1ff) as usize
}

const fn l3_index(virt_base: u64) -> usize {
    ((virt_base >> 12) & 0x1ff) as usize
}

const fn table_descriptor(table_phys_base: u64) -> u64 {
    (table_phys_base & DESCRIPTOR_OUTPUT_ADDRESS_MASK) | DESCRIPTOR_VALID | DESCRIPTOR_TABLE
}

const fn page_descriptor(mapping: Arm64PageMapping, phys_base: u64) -> u64 {
    (phys_base & DESCRIPTOR_OUTPUT_ADDRESS_MASK)
        | DESCRIPTOR_VALID
        | DESCRIPTOR_TABLE
        | (memory_attr_index(mapping.attr) << 2)
        | (access_perm_bits(mapping.access) << 6)
        | shareability_bits(mapping.attr)
        | DESCRIPTOR_AF
        | non_global_bit(mapping.access)
        | privileged_execute_never_bit(mapping)
        | user_execute_never_bit(mapping)
}

const fn memory_attr_index(attr: Arm64MemoryAttr) -> u64 {
    match attr {
        Arm64MemoryAttr::NormalCached => 0,
        Arm64MemoryAttr::DeviceNgNre => 1,
    }
}

const fn access_perm_bits(access: Arm64AccessPerm) -> u64 {
    match access {
        Arm64AccessPerm::KernelReadWrite => 0b00,
        Arm64AccessPerm::UserReadWrite => 0b01,
        Arm64AccessPerm::UserReadOnly | Arm64AccessPerm::UserReadExecute => 0b11,
    }
}

const fn shareability_bits(attr: Arm64MemoryAttr) -> u64 {
    match attr {
        Arm64MemoryAttr::NormalCached => 0b11 << 8,
        Arm64MemoryAttr::DeviceNgNre => 0b10 << 8,
    }
}

const fn non_global_bit(access: Arm64AccessPerm) -> u64 {
    match access {
        Arm64AccessPerm::KernelReadWrite => 0,
        Arm64AccessPerm::UserReadOnly
        | Arm64AccessPerm::UserReadWrite
        | Arm64AccessPerm::UserReadExecute => DESCRIPTOR_NG,
    }
}

const fn privileged_execute_never_bit(mapping: Arm64PageMapping) -> u64 {
    match mapping.access {
        Arm64AccessPerm::UserReadExecute => DESCRIPTOR_PXN,
        _ if mapping.execute_never => DESCRIPTOR_PXN,
        _ => 0,
    }
}

const fn user_execute_never_bit(mapping: Arm64PageMapping) -> u64 {
    if mapping.execute_never {
        DESCRIPTOR_UXN
    } else {
        0
    }
}

const DESCRIPTOR_VALID: u64 = 1 << 0;
const DESCRIPTOR_TABLE: u64 = 1 << 1;
const DESCRIPTOR_AF: u64 = 1 << 10;
const DESCRIPTOR_NG: u64 = 1 << 11;
const DESCRIPTOR_PXN: u64 = 1 << 53;
const DESCRIPTOR_UXN: u64 = 1 << 54;
const DESCRIPTOR_OUTPUT_ADDRESS_MASK: u64 = 0x0000_FFFF_FFFF_F000;

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

const fn u64_to_usize(value: u64) -> Option<usize> {
    if value > usize::MAX as u64 {
        None
    } else {
        Some(value as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Arm64AccessPerm, Arm64MemoryAttr, BOOTSTRAP_EL0_TRANSLATION_TABLE_BYTES,
        BOOTSTRAP_EL0_TRANSLATION_TABLE_ENTRIES, BootstrapEl0BackingFramePopulationReadiness,
        BootstrapEl0MappingReadiness, BootstrapEl0MappingRequest,
        BootstrapEl0PageTableConstructionReadiness, BootstrapEl0PageTableReadiness,
        BootstrapEl0PageTableRequest, DESCRIPTOR_NG, DESCRIPTOR_PXN, DESCRIPTOR_UXN,
        KERNEL_L2_TABLE_INDEX, L1_TABLE_INDEX, PAGE_SIZE, PageTablePlan, ROOT_TABLE_INDEX,
        USER_L2_TABLE_INDEX, USER_L3_TABLE_INDEX, construct_bootstrap_el0_page_tables, l0_index,
        l1_index, l2_index, l3_index, page_descriptor, populate_bootstrap_el0_backing_frames,
        table_descriptor, table_entry_offset,
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
        assert_eq!(plan.context_source_base, 0x8100_0000);
        assert_eq!(plan.context_source_size, 96);
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
            0x9000_B000,
            BOOTSTRAP_EL0_TRANSLATION_TABLE_BYTES,
        );

        let plan = mapping.page_table_plan(request);

        assert!(plan.ready());
        assert_eq!(plan.readiness, BootstrapEl0PageTableReadiness::Ready);
        assert_eq!(plan.source_readiness, BootstrapEl0MappingReadiness::Ready);
        assert_eq!(plan.payload_copy_source_base, 0x8020_0098);
        assert_eq!(plan.payload_copy_source_size, 0x1234);
        assert_eq!(plan.context_copy_source_base, 0x8100_0000);
        assert_eq!(plan.context_copy_source_size, 96);
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
        assert_eq!(plan.page_table_phys_base, 0x9000_B000);
        assert_eq!(plan.page_table_bytes, BOOTSTRAP_EL0_TRANSLATION_TABLE_BYTES);
        assert_eq!(plan.root_table_phys_base, 0x9000_B000);
        assert_eq!(plan.l1_table_phys_base, 0x9000_C000);
        assert_eq!(plan.kernel_l2_table_phys_base, 0x9000_D000);
        assert_eq!(plan.kernel_l3_table_phys_base, 0x9000_E000);
        assert_eq!(plan.user_l2_table_phys_base, 0x9000_F000);
        assert_eq!(plan.user_l3_table_phys_base, 0x9001_0000);
    }

    #[test]
    fn bootstrap_el0_page_table_construction_writes_translation_tables() {
        let plan = ready_bootstrap_el0_page_table_plan();
        let mut entries = [u64::MAX; BOOTSTRAP_EL0_TRANSLATION_TABLE_ENTRIES];

        let construction = construct_bootstrap_el0_page_tables(plan, &mut entries);

        assert!(construction.ready());
        assert_eq!(
            construction.readiness,
            BootstrapEl0PageTableConstructionReadiness::Ready
        );
        assert_eq!(
            construction.source_readiness,
            BootstrapEl0PageTableReadiness::Ready
        );
        assert_eq!(construction.root_table_phys_base, plan.root_table_phys_base);
        assert_eq!(construction.page_table_bytes, plan.page_table_bytes);
        assert_eq!(construction.mapped_pages, 16);
        assert_eq!(
            entries[table_entry_offset(
                ROOT_TABLE_INDEX,
                l0_index(plan.user_image_mapping.virt_base)
            )],
            table_descriptor(plan.l1_table_phys_base)
        );
        assert_eq!(
            entries[table_entry_offset(L1_TABLE_INDEX, l1_index(plan.kernel_mapping.virt_base))],
            table_descriptor(plan.kernel_l2_table_phys_base)
        );
        assert_eq!(
            entries
                [table_entry_offset(L1_TABLE_INDEX, l1_index(plan.user_image_mapping.virt_base))],
            table_descriptor(plan.user_l2_table_phys_base)
        );
        assert_eq!(
            entries[table_entry_offset(
                KERNEL_L2_TABLE_INDEX,
                l2_index(plan.kernel_mapping.virt_base)
            )],
            table_descriptor(plan.kernel_l3_table_phys_base)
        );
        assert_eq!(
            entries[table_entry_offset(
                USER_L2_TABLE_INDEX,
                l2_index(plan.user_image_mapping.virt_base)
            )],
            table_descriptor(plan.user_l3_table_phys_base)
        );

        let image_entry = entries[table_entry_offset(
            USER_L3_TABLE_INDEX,
            l3_index(plan.user_image_mapping.virt_base),
        )];
        assert_eq!(
            image_entry,
            page_descriptor(plan.user_image_mapping, plan.user_image_mapping.phys_base)
        );
        assert_ne!(image_entry & DESCRIPTOR_NG, 0);
        assert_ne!(image_entry & DESCRIPTOR_PXN, 0);
        assert_eq!(image_entry & DESCRIPTOR_UXN, 0);

        let context_entry = entries[table_entry_offset(
            USER_L3_TABLE_INDEX,
            l3_index(plan.user_context_mapping.virt_base),
        )];
        assert_eq!(
            context_entry,
            page_descriptor(
                plan.user_context_mapping,
                plan.user_context_mapping.phys_base
            )
        );
        assert_ne!(context_entry & DESCRIPTOR_PXN, 0);
        assert_ne!(context_entry & DESCRIPTOR_UXN, 0);

        let stack_entry = entries[table_entry_offset(
            USER_L3_TABLE_INDEX,
            l3_index(plan.user_stack_mapping.virt_base),
        )];
        assert_eq!(
            stack_entry,
            page_descriptor(plan.user_stack_mapping, plan.user_stack_mapping.phys_base)
        );
        assert_eq!((stack_entry >> 6) & 0b11, 0b01);
    }

    #[test]
    fn bootstrap_el0_page_table_plan_rejects_small_table_arena() {
        let mapping = ready_bootstrap_el0_mapping_plan();
        let request = BootstrapEl0PageTableRequest::new(
            0x1000_0000,
            0x5000,
            0x9000_0000,
            0x9000_2000,
            0x9000_3000,
            0x9000_B000,
            BOOTSTRAP_EL0_TRANSLATION_TABLE_BYTES - PAGE_SIZE,
        );

        let plan = mapping.page_table_plan(request);

        assert_eq!(
            plan.readiness,
            BootstrapEl0PageTableReadiness::PageTableArenaTooSmall
        );
    }

    #[test]
    fn bootstrap_el0_page_table_construction_refuses_short_output_buffer() {
        let plan = ready_bootstrap_el0_page_table_plan();
        let mut entries = [0; BOOTSTRAP_EL0_TRANSLATION_TABLE_ENTRIES - 1];

        let construction = construct_bootstrap_el0_page_tables(plan, &mut entries);

        assert_eq!(
            construction.readiness,
            BootstrapEl0PageTableConstructionReadiness::OutputBufferTooSmall
        );
        assert_eq!(
            construction.source_readiness,
            BootstrapEl0PageTableReadiness::Ready
        );
    }

    #[test]
    fn bootstrap_el0_backing_frame_population_copies_payload_context_and_zeros_padding() {
        let plan = ready_bootstrap_el0_page_table_plan();
        let payload_source = [0x5a; 0x1234];
        let context_source = [0xc3; 96];
        let mut image_frame = [0xaa; 0x2000];
        let mut context_frame = [0xbb; 0x1000];
        let mut stack_frame = [0xcc; 0x8000];

        let population = populate_bootstrap_el0_backing_frames(
            plan,
            &payload_source,
            &context_source,
            &mut image_frame,
            &mut context_frame,
            &mut stack_frame,
        );

        assert!(population.ready());
        assert_eq!(
            population.readiness,
            BootstrapEl0BackingFramePopulationReadiness::Ready
        );
        assert_eq!(population.payload_bytes, 0x1234);
        assert_eq!(population.context_bytes, 96);
        assert_eq!(
            population.zeroed_bytes,
            0x8000 + 0x2000 - 0x1234 + 0x1000 - 96
        );
        assert_eq!(&image_frame[..0x1234], &payload_source);
        assert!(image_frame[0x1234..].iter().all(|byte| *byte == 0));
        assert_eq!(&context_frame[..96], &context_source);
        assert!(context_frame[96..].iter().all(|byte| *byte == 0));
        assert!(stack_frame.iter().all(|byte| *byte == 0));
    }

    #[test]
    fn bootstrap_el0_backing_frame_population_rejects_short_payload_source() {
        let plan = ready_bootstrap_el0_page_table_plan();
        let payload_source = [0x5a; 0x1233];
        let context_source = [0xc3; 96];
        let mut image_frame = [0xaa; 0x2000];
        let mut context_frame = [0xbb; 0x1000];
        let mut stack_frame = [0xcc; 0x8000];

        let population = populate_bootstrap_el0_backing_frames(
            plan,
            &payload_source,
            &context_source,
            &mut image_frame,
            &mut context_frame,
            &mut stack_frame,
        );

        assert_eq!(
            population.readiness,
            BootstrapEl0BackingFramePopulationReadiness::PayloadSourceTooSmall
        );
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
            0x9000_B000,
            BOOTSTRAP_EL0_TRANSLATION_TABLE_BYTES,
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
            0x9000_B000,
            BOOTSTRAP_EL0_TRANSLATION_TABLE_BYTES,
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

    fn ready_bootstrap_el0_page_table_plan() -> super::BootstrapEl0PageTablePlan {
        ready_bootstrap_el0_mapping_plan().page_table_plan(BootstrapEl0PageTableRequest::new(
            0x1000_0000,
            0x5000,
            0x9000_0000,
            0x9000_2000,
            0x9000_3000,
            0x9000_B000,
            BOOTSTRAP_EL0_TRANSLATION_TABLE_BYTES,
        ))
    }
}
