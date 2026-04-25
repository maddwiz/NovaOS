use nova_rt::{
    NovaServiceArtifactSpec, NovaServiceBootstrapRequirement, NovaServiceDescriptor, NovaServiceId,
    NovaServiceKind, NovaServiceLaunchSpec,
};

pub const ACCELD_DESCRIPTOR: NovaServiceDescriptor = NovaServiceDescriptor::new(
    NovaServiceId::ACCELD,
    "acceld",
    NovaServiceKind::Core,
    true,
    40,
);

pub const ACCELD_LAUNCH_SPEC: NovaServiceLaunchSpec = NovaServiceLaunchSpec::new(
    ACCELD_DESCRIPTOR,
    NovaServiceBootstrapRequirement::core_required(),
)
.with_artifact(ACCELD_PAYLOAD_SPEC);

pub const ACCELD_PAYLOAD_SPEC: NovaServiceArtifactSpec =
    NovaServiceArtifactSpec::service_payload("acceld-payload");
