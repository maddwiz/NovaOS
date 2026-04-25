use nova_rt::{
    NovaServiceArtifactSpec, NovaServiceBootstrapRequirement, NovaServiceDescriptor, NovaServiceId,
    NovaServiceKind, NovaServiceLaunchSpec,
};

pub const MEMD_DESCRIPTOR: NovaServiceDescriptor =
    NovaServiceDescriptor::new(NovaServiceId::MEMD, "memd", NovaServiceKind::Core, true, 30);

pub const MEMD_LAUNCH_SPEC: NovaServiceLaunchSpec = NovaServiceLaunchSpec::new(
    MEMD_DESCRIPTOR,
    NovaServiceBootstrapRequirement::core_required(),
)
.with_artifact(MEMD_PAYLOAD_SPEC);

pub const MEMD_PAYLOAD_SPEC: NovaServiceArtifactSpec =
    NovaServiceArtifactSpec::service_payload("memd-payload");
