use nova_rt::{
    NovaServiceBootstrapRequirement, NovaServiceDescriptor, NovaServiceId, NovaServiceKind,
    NovaServiceLaunchSpec,
};

pub const SCENED_DESCRIPTOR: NovaServiceDescriptor = NovaServiceDescriptor::new(
    NovaServiceId::SCENED,
    "scened",
    NovaServiceKind::Interaction,
    true,
    60,
);

pub const SCENED_LAUNCH_SPEC: NovaServiceLaunchSpec = NovaServiceLaunchSpec::new(
    SCENED_DESCRIPTOR,
    NovaServiceBootstrapRequirement::core_required(),
);
