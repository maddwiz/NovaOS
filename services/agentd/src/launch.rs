use nova_rt::{
    NovaServiceArtifactSpec, NovaServiceBootstrapRequirement, NovaServiceDescriptor, NovaServiceId,
    NovaServiceKind, NovaServiceLaunchSpec,
};

pub const AGENTD_DESCRIPTOR: NovaServiceDescriptor = NovaServiceDescriptor::new(
    NovaServiceId::AGENTD,
    "agentd",
    NovaServiceKind::Core,
    true,
    20,
);

pub const AGENTD_LAUNCH_SPEC: NovaServiceLaunchSpec = NovaServiceLaunchSpec::new(
    AGENTD_DESCRIPTOR,
    NovaServiceBootstrapRequirement::core_required(),
)
.with_artifact(AGENTD_PAYLOAD_SPEC);

pub const AGENTD_PAYLOAD_SPEC: NovaServiceArtifactSpec =
    NovaServiceArtifactSpec::service_payload("agentd-payload");
