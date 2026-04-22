use nova_rt::{
    NovaEndpointId, NovaInitCapsuleCapabilityV1, NovaSceneId, NovaServiceBootstrapRequirement,
    NovaServiceDescriptor, NovaServiceId, NovaServiceKernelBinding, NovaServiceKernelLaunchPlan,
    NovaServiceKind, NovaServiceLaunchRequest, NovaServiceLaunchSpec, NovaServiceStatus,
    NovaSharedMemoryRegionId, NovaTaskId,
};

pub const POLICYD_DESCRIPTOR: NovaServiceDescriptor = NovaServiceDescriptor::new(
    NovaServiceId::POLICYD,
    "policyd",
    NovaServiceKind::Core,
    true,
    10,
);
pub const AGENTD_DESCRIPTOR: NovaServiceDescriptor = NovaServiceDescriptor::new(
    NovaServiceId::AGENTD,
    "agentd",
    NovaServiceKind::Core,
    true,
    20,
);
pub const MEMD_DESCRIPTOR: NovaServiceDescriptor =
    NovaServiceDescriptor::new(NovaServiceId::MEMD, "memd", NovaServiceKind::Core, true, 30);
pub const ACCELD_DESCRIPTOR: NovaServiceDescriptor = NovaServiceDescriptor::new(
    NovaServiceId::ACCELD,
    "acceld",
    NovaServiceKind::Core,
    true,
    40,
);
pub const INTENTD_DESCRIPTOR: NovaServiceDescriptor = NovaServiceDescriptor::new(
    NovaServiceId::INTENTD,
    "intentd",
    NovaServiceKind::Interaction,
    true,
    50,
);
pub const SCENED_DESCRIPTOR: NovaServiceDescriptor = NovaServiceDescriptor::new(
    NovaServiceId::SCENED,
    "scened",
    NovaServiceKind::Interaction,
    true,
    60,
);
pub const APPBRIDGED_DESCRIPTOR: NovaServiceDescriptor = NovaServiceDescriptor::new(
    NovaServiceId::APPBRIDGED,
    "appbridged",
    NovaServiceKind::Bridge,
    true,
    70,
);
pub const SHELLD_DESCRIPTOR: NovaServiceDescriptor = NovaServiceDescriptor::new(
    NovaServiceId::SHELLD,
    "shelld",
    NovaServiceKind::Operator,
    false,
    80,
);

pub const CORE_SERVICE_LAUNCH_ORDER: &[NovaServiceDescriptor] = &[
    POLICYD_DESCRIPTOR,
    AGENTD_DESCRIPTOR,
    MEMD_DESCRIPTOR,
    ACCELD_DESCRIPTOR,
    INTENTD_DESCRIPTOR,
    SCENED_DESCRIPTOR,
    APPBRIDGED_DESCRIPTOR,
    SHELLD_DESCRIPTOR,
];

pub const CORE_SERVICE_BOOT_STATUSES: &[NovaServiceStatus] = &[
    NovaServiceStatus::running(POLICYD_DESCRIPTOR),
    NovaServiceStatus::running(AGENTD_DESCRIPTOR),
    NovaServiceStatus::running(MEMD_DESCRIPTOR),
    NovaServiceStatus::running(ACCELD_DESCRIPTOR),
    NovaServiceStatus::running(INTENTD_DESCRIPTOR),
    NovaServiceStatus::running(SCENED_DESCRIPTOR),
    NovaServiceStatus::running(APPBRIDGED_DESCRIPTOR),
    NovaServiceStatus::deferred(SHELLD_DESCRIPTOR, 1),
];

const REQUIRED_SERVICE_CAPS: u64 = NovaInitCapsuleCapabilityV1::BootLog as u64
    | NovaInitCapsuleCapabilityV1::Yield as u64
    | NovaInitCapsuleCapabilityV1::EndpointBootstrap as u64
    | NovaInitCapsuleCapabilityV1::SharedMemoryBootstrap as u64;
const OPTIONAL_SERVICE_CAPS: u64 = NovaInitCapsuleCapabilityV1::BootLog as u64;

const REQUIRED_SERVICE_BOOTSTRAP: NovaServiceBootstrapRequirement =
    NovaServiceBootstrapRequirement::new(REQUIRED_SERVICE_CAPS, 1, 1);
const OPTIONAL_SERVICE_BOOTSTRAP: NovaServiceBootstrapRequirement =
    NovaServiceBootstrapRequirement::new(OPTIONAL_SERVICE_CAPS, 0, 0);

pub const CORE_SERVICE_LAUNCH_SPECS: &[NovaServiceLaunchSpec] = &[
    NovaServiceLaunchSpec::new(POLICYD_DESCRIPTOR, REQUIRED_SERVICE_BOOTSTRAP),
    NovaServiceLaunchSpec::new(AGENTD_DESCRIPTOR, REQUIRED_SERVICE_BOOTSTRAP),
    NovaServiceLaunchSpec::new(MEMD_DESCRIPTOR, REQUIRED_SERVICE_BOOTSTRAP),
    NovaServiceLaunchSpec::new(ACCELD_DESCRIPTOR, REQUIRED_SERVICE_BOOTSTRAP),
    NovaServiceLaunchSpec::new(INTENTD_DESCRIPTOR, REQUIRED_SERVICE_BOOTSTRAP),
    NovaServiceLaunchSpec::new(SCENED_DESCRIPTOR, REQUIRED_SERVICE_BOOTSTRAP),
    NovaServiceLaunchSpec::new(APPBRIDGED_DESCRIPTOR, REQUIRED_SERVICE_BOOTSTRAP),
    NovaServiceLaunchSpec::new(SHELLD_DESCRIPTOR, OPTIONAL_SERVICE_BOOTSTRAP),
];

const POLICYD_KERNEL_BINDING: NovaServiceKernelBinding = NovaServiceKernelBinding::planned(
    NovaServiceId::POLICYD,
    NovaTaskId::new(0x1001),
    NovaEndpointId::new(0x2001),
    NovaSharedMemoryRegionId::new(0x3001),
);
const AGENTD_KERNEL_BINDING: NovaServiceKernelBinding = NovaServiceKernelBinding::planned(
    NovaServiceId::AGENTD,
    NovaTaskId::new(0x1002),
    NovaEndpointId::new(0x2002),
    NovaSharedMemoryRegionId::new(0x3002),
);
const MEMD_KERNEL_BINDING: NovaServiceKernelBinding = NovaServiceKernelBinding::planned(
    NovaServiceId::MEMD,
    NovaTaskId::new(0x1003),
    NovaEndpointId::new(0x2003),
    NovaSharedMemoryRegionId::new(0x3003),
);
const ACCELD_KERNEL_BINDING: NovaServiceKernelBinding = NovaServiceKernelBinding::planned(
    NovaServiceId::ACCELD,
    NovaTaskId::new(0x1004),
    NovaEndpointId::new(0x2004),
    NovaSharedMemoryRegionId::new(0x3004),
);
const INTENTD_KERNEL_BINDING: NovaServiceKernelBinding = NovaServiceKernelBinding::planned(
    NovaServiceId::INTENTD,
    NovaTaskId::new(0x1005),
    NovaEndpointId::new(0x2005),
    NovaSharedMemoryRegionId::new(0x3005),
);
const SCENED_KERNEL_BINDING: NovaServiceKernelBinding = NovaServiceKernelBinding::planned(
    NovaServiceId::SCENED,
    NovaTaskId::new(0x1006),
    NovaEndpointId::new(0x2006),
    NovaSharedMemoryRegionId::new(0x3006),
);
const APPBRIDGED_KERNEL_BINDING: NovaServiceKernelBinding = NovaServiceKernelBinding::planned(
    NovaServiceId::APPBRIDGED,
    NovaTaskId::new(0x1007),
    NovaEndpointId::new(0x2007),
    NovaSharedMemoryRegionId::new(0x3007),
);
const SHELLD_KERNEL_BINDING: NovaServiceKernelBinding =
    NovaServiceKernelBinding::model_only(NovaServiceId::SHELLD);

pub const CORE_SERVICE_KERNEL_LAUNCH_PLANS: &[NovaServiceKernelLaunchPlan] = &[
    NovaServiceKernelLaunchPlan::new(
        POLICYD_DESCRIPTOR,
        service_launch_request(NovaServiceId::POLICYD),
        POLICYD_KERNEL_BINDING,
    ),
    NovaServiceKernelLaunchPlan::new(
        AGENTD_DESCRIPTOR,
        service_launch_request(NovaServiceId::AGENTD),
        AGENTD_KERNEL_BINDING,
    ),
    NovaServiceKernelLaunchPlan::new(
        MEMD_DESCRIPTOR,
        service_launch_request(NovaServiceId::MEMD),
        MEMD_KERNEL_BINDING,
    ),
    NovaServiceKernelLaunchPlan::new(
        ACCELD_DESCRIPTOR,
        service_launch_request(NovaServiceId::ACCELD),
        ACCELD_KERNEL_BINDING,
    ),
    NovaServiceKernelLaunchPlan::new(
        INTENTD_DESCRIPTOR,
        service_launch_request(NovaServiceId::INTENTD),
        INTENTD_KERNEL_BINDING,
    ),
    NovaServiceKernelLaunchPlan::new(
        SCENED_DESCRIPTOR,
        service_launch_request(NovaServiceId::SCENED),
        SCENED_KERNEL_BINDING,
    ),
    NovaServiceKernelLaunchPlan::new(
        APPBRIDGED_DESCRIPTOR,
        service_launch_request(NovaServiceId::APPBRIDGED),
        APPBRIDGED_KERNEL_BINDING,
    ),
    NovaServiceKernelLaunchPlan::new(
        SHELLD_DESCRIPTOR,
        service_launch_request(NovaServiceId::SHELLD),
        SHELLD_KERNEL_BINDING,
    ),
];

const fn service_launch_request(target: NovaServiceId) -> NovaServiceLaunchRequest {
    NovaServiceLaunchRequest::new(initd_descriptor().id, target, NovaSceneId::ROOT, 0)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InitServiceLaunchTable {
    pub init_service: NovaServiceDescriptor,
    pub services: &'static [NovaServiceDescriptor],
}

impl InitServiceLaunchTable {
    pub const fn new(services: &'static [NovaServiceDescriptor]) -> Self {
        Self {
            init_service: initd_descriptor(),
            services,
        }
    }

    pub const fn service_count(self) -> usize {
        self.services.len()
    }

    pub fn required_service_count(self) -> usize {
        self.services
            .iter()
            .filter(|service| service.required)
            .count()
    }

    pub fn launch_request(self, index: usize) -> Option<NovaServiceLaunchRequest> {
        let target = self.services.get(index)?;
        Some(service_launch_request(target.id))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InitServiceLaunchPlan {
    pub init_service: NovaServiceDescriptor,
    pub specs: &'static [NovaServiceLaunchSpec],
}

impl InitServiceLaunchPlan {
    pub const fn new(specs: &'static [NovaServiceLaunchSpec]) -> Self {
        Self {
            init_service: initd_descriptor(),
            specs,
        }
    }

    pub const fn service_count(self) -> usize {
        self.specs.len()
    }

    pub fn required_service_count(self) -> usize {
        self.specs
            .iter()
            .filter(|spec| spec.descriptor.required)
            .count()
    }

    pub fn spec_for(self, id: NovaServiceId) -> Option<NovaServiceLaunchSpec> {
        self.specs
            .iter()
            .copied()
            .find(|spec| spec.descriptor.id == id)
    }

    pub fn launch_request_for(self, id: NovaServiceId) -> Option<NovaServiceLaunchRequest> {
        Some(
            self.spec_for(id)?
                .launch_request(self.init_service.id, NovaSceneId::ROOT),
        )
    }

    pub fn validate(self) -> bool {
        if self.specs.is_empty() {
            return false;
        }

        let mut index = 0usize;
        while index < self.specs.len() {
            let spec = self.specs[index];
            if !spec.is_valid() {
                return false;
            }
            if index > 0
                && self.specs[index - 1].descriptor.launch_order >= spec.descriptor.launch_order
            {
                return false;
            }

            let mut compare = index + 1;
            while compare < self.specs.len() {
                if self.specs[compare].descriptor.id == spec.descriptor.id {
                    return false;
                }
                compare += 1;
            }
            index += 1;
        }

        true
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InitRuntimeSnapshot {
    pub registered_service: NovaServiceDescriptor,
    pub launch_service_count: u16,
    pub required_service_count: u16,
    pub health_generation: u64,
}

impl InitRuntimeSnapshot {
    pub const fn healthy(self) -> bool {
        self.launch_service_count >= self.required_service_count && self.health_generation != 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InitRuntimeStatusPage {
    pub registered_service: NovaServiceDescriptor,
    pub services: &'static [NovaServiceStatus],
    pub health_generation: u64,
}

impl InitRuntimeStatusPage {
    pub const fn service_count(self) -> usize {
        self.services.len()
    }

    pub fn required_service_count(self) -> usize {
        self.services
            .iter()
            .filter(|status| status.descriptor.required)
            .count()
    }

    pub fn running_required_service_count(self) -> usize {
        self.services
            .iter()
            .filter(|status| status.descriptor.required && status.is_healthy())
            .count()
    }

    pub fn status_for(self, id: NovaServiceId) -> Option<NovaServiceStatus> {
        self.services
            .iter()
            .copied()
            .find(|status| status.descriptor.id == id)
    }

    pub fn healthy(self) -> bool {
        self.health_generation != 0
            && self.running_required_service_count() >= self.required_service_count()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InitKernelLaunchPlanPage {
    pub registered_service: NovaServiceDescriptor,
    pub plans: &'static [NovaServiceKernelLaunchPlan],
    pub generation: u64,
}

impl InitKernelLaunchPlanPage {
    pub const fn service_count(self) -> usize {
        self.plans.len()
    }

    pub fn required_service_count(self) -> usize {
        self.plans
            .iter()
            .filter(|plan| plan.descriptor.required)
            .count()
    }

    pub fn planned_required_service_count(self) -> usize {
        self.plans
            .iter()
            .filter(|plan| plan.descriptor.required && plan.binding.has_kernel_objects())
            .count()
    }

    pub fn kernel_backed_service_count(self) -> usize {
        self.plans
            .iter()
            .filter(|plan| plan.binding.can_publish_kernel_health())
            .count()
    }

    pub fn plan_for(self, id: NovaServiceId) -> Option<NovaServiceKernelLaunchPlan> {
        self.plans
            .iter()
            .copied()
            .find(|plan| plan.descriptor.id == id)
    }

    pub fn ready_for_kernel_handoff(self) -> bool {
        self.generation != 0
            && self.planned_required_service_count() >= self.required_service_count()
    }
}

pub const fn initd_descriptor() -> NovaServiceDescriptor {
    NovaServiceDescriptor::new(
        NovaServiceId::INITD,
        "initd",
        NovaServiceKind::Core,
        true,
        0,
    )
}

pub const fn core_launch_table() -> InitServiceLaunchTable {
    InitServiceLaunchTable::new(CORE_SERVICE_LAUNCH_ORDER)
}

pub const fn core_launch_plan() -> InitServiceLaunchPlan {
    InitServiceLaunchPlan::new(CORE_SERVICE_LAUNCH_SPECS)
}

pub const fn initd_boot_status_page() -> InitRuntimeStatusPage {
    InitRuntimeStatusPage {
        registered_service: initd_descriptor(),
        services: CORE_SERVICE_BOOT_STATUSES,
        health_generation: 1,
    }
}

pub const fn initd_kernel_launch_plan_page() -> InitKernelLaunchPlanPage {
    InitKernelLaunchPlanPage {
        registered_service: initd_descriptor(),
        plans: CORE_SERVICE_KERNEL_LAUNCH_PLANS,
        generation: 1,
    }
}

pub fn initd_boot_snapshot() -> InitRuntimeSnapshot {
    let table = core_launch_table();
    let status_page = initd_boot_status_page();
    InitRuntimeSnapshot {
        registered_service: table.init_service,
        launch_service_count: table.service_count() as u16,
        required_service_count: status_page.required_service_count() as u16,
        health_generation: status_page.health_generation,
    }
}
