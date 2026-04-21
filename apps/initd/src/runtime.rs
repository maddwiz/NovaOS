use nova_rt::{
    NovaSceneId, NovaServiceDescriptor, NovaServiceId, NovaServiceKind, NovaServiceLaunchRequest,
};

pub const CORE_SERVICE_LAUNCH_ORDER: &[NovaServiceDescriptor] = &[
    NovaServiceDescriptor::new(
        NovaServiceId::POLICYD,
        "policyd",
        NovaServiceKind::Core,
        true,
        10,
    ),
    NovaServiceDescriptor::new(
        NovaServiceId::AGENTD,
        "agentd",
        NovaServiceKind::Core,
        true,
        20,
    ),
    NovaServiceDescriptor::new(NovaServiceId::MEMD, "memd", NovaServiceKind::Core, true, 30),
    NovaServiceDescriptor::new(
        NovaServiceId::ACCELD,
        "acceld",
        NovaServiceKind::Core,
        true,
        40,
    ),
    NovaServiceDescriptor::new(
        NovaServiceId::INTENTD,
        "intentd",
        NovaServiceKind::Interaction,
        true,
        50,
    ),
    NovaServiceDescriptor::new(
        NovaServiceId::SCENED,
        "scened",
        NovaServiceKind::Interaction,
        true,
        60,
    ),
    NovaServiceDescriptor::new(
        NovaServiceId::APPBRIDGED,
        "appbridged",
        NovaServiceKind::Bridge,
        true,
        70,
    ),
    NovaServiceDescriptor::new(
        NovaServiceId::SHELLD,
        "shelld",
        NovaServiceKind::Operator,
        false,
        80,
    ),
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

pub fn initd_boot_snapshot() -> InitRuntimeSnapshot {
    let table = core_launch_table();
    InitRuntimeSnapshot {
        registered_service: table.init_service,
        launch_service_count: table.service_count() as u16,
        required_service_count: table.required_service_count() as u16,
        health_generation: 1,
    }
}
