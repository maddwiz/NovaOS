use nova_rt::{
    NovaServiceBootstrapRequirement, NovaServiceDescriptor, NovaServiceId, NovaServiceKind,
    NovaServiceLaunchSpec,
};

pub const APPBRIDGED_DESCRIPTOR: NovaServiceDescriptor = NovaServiceDescriptor::new(
    NovaServiceId::APPBRIDGED,
    "appbridged",
    NovaServiceKind::Bridge,
    true,
    70,
);

pub const APPBRIDGED_LAUNCH_SPEC: NovaServiceLaunchSpec = NovaServiceLaunchSpec::new(
    APPBRIDGED_DESCRIPTOR,
    NovaServiceBootstrapRequirement::core_required(),
);
