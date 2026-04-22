use nova_rt::{
    NovaServiceBootstrapRequirement, NovaServiceDescriptor, NovaServiceId, NovaServiceKind,
    NovaServiceLaunchSpec,
};

pub const AGENTD_DESCRIPTOR: NovaServiceDescriptor = NovaServiceDescriptor::new(
    NovaServiceId::AGENTD,
    "agentd",
    NovaServiceKind::Core,
    true,
    20,
);

pub const AGENTD_LAUNCH_SPEC: NovaServiceLaunchSpec = NovaServiceLaunchSpec::new(
    AGENTD_DESCRIPTOR,
    NovaServiceBootstrapRequirement::core_required(),
);
