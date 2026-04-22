use nova_rt::{
    NovaServiceBootstrapRequirement, NovaServiceDescriptor, NovaServiceId, NovaServiceKind,
    NovaServiceLaunchSpec,
};

pub const SHELLD_DESCRIPTOR: NovaServiceDescriptor = NovaServiceDescriptor::new(
    NovaServiceId::SHELLD,
    "shelld",
    NovaServiceKind::Operator,
    false,
    80,
);

pub const SHELLD_LAUNCH_SPEC: NovaServiceLaunchSpec = NovaServiceLaunchSpec::new(
    SHELLD_DESCRIPTOR,
    NovaServiceBootstrapRequirement::boot_log_only(),
);
