use nova_rt::{
    NovaServiceArtifactSpec, NovaServiceBootstrapRequirement, NovaServiceDescriptor, NovaServiceId,
    NovaServiceKind, NovaServiceLaunchSpec,
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
)
.with_artifact(APPBRIDGED_PAYLOAD_SPEC);

pub const APPBRIDGED_PAYLOAD_SPEC: NovaServiceArtifactSpec =
    NovaServiceArtifactSpec::service_payload("appbridged-payload");
