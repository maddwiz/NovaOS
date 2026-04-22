use nova_rt::{
    NovaServiceBootstrapRequirement, NovaServiceDescriptor, NovaServiceId, NovaServiceKind,
    NovaServiceLaunchSpec,
};

pub const POLICYD_DESCRIPTOR: NovaServiceDescriptor = NovaServiceDescriptor::new(
    NovaServiceId::POLICYD,
    "policyd",
    NovaServiceKind::Core,
    true,
    10,
);

pub const POLICYD_LAUNCH_SPEC: NovaServiceLaunchSpec = NovaServiceLaunchSpec::new(
    POLICYD_DESCRIPTOR,
    NovaServiceBootstrapRequirement::core_required(),
);
