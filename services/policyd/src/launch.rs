use nova_rt::{
    NovaServiceArtifactSpec, NovaServiceBootstrapRequirement, NovaServiceDescriptor, NovaServiceId,
    NovaServiceKind, NovaServiceLaunchSpec,
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
)
.with_artifact(POLICYD_PAYLOAD_SPEC);

pub const POLICYD_PAYLOAD_SPEC: NovaServiceArtifactSpec =
    NovaServiceArtifactSpec::service_payload("policyd-payload");
