use nova_rt::{
    NovaServiceBootstrapRequirement, NovaServiceDescriptor, NovaServiceId, NovaServiceKind,
    NovaServiceLaunchSpec,
};

pub const INTENTD_DESCRIPTOR: NovaServiceDescriptor = NovaServiceDescriptor::new(
    NovaServiceId::INTENTD,
    "intentd",
    NovaServiceKind::Interaction,
    true,
    50,
);

pub const INTENTD_LAUNCH_SPEC: NovaServiceLaunchSpec = NovaServiceLaunchSpec::new(
    INTENTD_DESCRIPTOR,
    NovaServiceBootstrapRequirement::core_required(),
);
