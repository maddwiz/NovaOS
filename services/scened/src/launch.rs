use nova_rt::{
    NovaServiceArtifactSpec, NovaServiceBootstrapRequirement, NovaServiceDescriptor, NovaServiceId,
    NovaServiceKind, NovaServiceLaunchSpec,
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
)
.with_artifact(SCENED_PAYLOAD_SPEC);

pub const SCENED_PAYLOAD_SPEC: NovaServiceArtifactSpec =
    NovaServiceArtifactSpec::service_payload("scened-payload");
