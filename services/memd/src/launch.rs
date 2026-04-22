use nova_rt::{
    NovaServiceBootstrapRequirement, NovaServiceDescriptor, NovaServiceId, NovaServiceKind,
    NovaServiceLaunchSpec,
};

pub const MEMD_DESCRIPTOR: NovaServiceDescriptor =
    NovaServiceDescriptor::new(NovaServiceId::MEMD, "memd", NovaServiceKind::Core, true, 30);

pub const MEMD_LAUNCH_SPEC: NovaServiceLaunchSpec = NovaServiceLaunchSpec::new(
    MEMD_DESCRIPTOR,
    NovaServiceBootstrapRequirement::core_required(),
);
