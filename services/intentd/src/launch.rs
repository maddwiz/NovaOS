use nova_rt::{
    NovaServiceArtifactSpec, NovaServiceBootstrapRequirement, NovaServiceDescriptor, NovaServiceId,
    NovaServiceKind, NovaServiceLaunchSpec,
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
)
.with_artifact(INTENTD_PAYLOAD_SPEC);

pub const INTENTD_PAYLOAD_SPEC: NovaServiceArtifactSpec =
    NovaServiceArtifactSpec::service_payload("intentd-payload");
