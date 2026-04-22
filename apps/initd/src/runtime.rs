use nova_rt::{
    NovaSceneId, NovaServiceDescriptor, NovaServiceId, NovaServiceKind, NovaServiceLaunchRequest,
    NovaServiceStatus,
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
        Some(NovaServiceLaunchRequest {
            requester: self.init_service.id,
            target: target.id,
            scene: NovaSceneId::ROOT,
            flags: 0,
        })
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

pub const fn initd_boot_status_page() -> InitRuntimeStatusPage {
    InitRuntimeStatusPage {
        registered_service: initd_descriptor(),
        services: CORE_SERVICE_BOOT_STATUSES,
        health_generation: 1,
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
