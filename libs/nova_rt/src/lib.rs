#![no_std]

#[cfg(test)]
extern crate alloc;

pub mod bootinfo;
pub mod bootinfo_v2;
pub mod bootstrap_task;
pub mod digest;
pub mod init_capsule;
pub mod payload;
pub mod service;
pub mod syscall;
pub mod verification;

pub use bootinfo::{
    BootSource, BootSummary, FirmwareInfo, FramebufferFormat, FramebufferInfo, MemoryInfo,
    NovaBootInfoV1,
};
pub use bootinfo_v2::{
    NovaBootInfoV2, NovaBootstrapFrameArenaDescriptorV1, NovaBootstrapPayloadDescriptorV1,
    NovaBootstrapUserWindowDescriptorV1, NovaDisplayPathDescriptorV1, NovaFramebufferDescriptorV1,
    NovaNetworkSeedV1, NovaStorageSeedV1,
};
pub use bootstrap_task::{
    NovaBootstrapKernelCallEntryV1, NovaBootstrapTaskContextV1, NovaBootstrapTaskContextV2,
    ResolvedBootstrapTaskContext, bootstrap_kernel_call, bootstrap_trace,
    resolve_bootstrap_task_context,
};
pub use digest::{NovaDigestAlgorithm, NovaImageDigestV1, sha256_digest_bytes};
pub use init_capsule::{
    InitCapsuleImage, NOVA_INIT_CAPSULE_KNOWN_CAPABILITIES_V1, NOVA_INIT_CAPSULE_SERVICE_NAME_LEN,
    NovaInitCapsuleCapabilityV1, NovaInitCapsuleHeaderV1, decode_init_capsule_service_name,
    encode_init_capsule_service_name,
};
pub use payload::{
    NovaPayloadEntryAbi, NovaPayloadHeaderV1, NovaPayloadKind, NovaPayloadLoadMode, PayloadImage,
};
pub use service::{
    NovaAgentId, NovaAppActionKind, NovaAppActionRequest, NovaAppBridgeKind, NovaAppDescriptor,
    NovaAppId, NovaEndpointId, NovaIntentDispatch, NovaIntentEnvelope, NovaIntentKind,
    NovaIntentProjection, NovaPolicyAction, NovaPolicyDecision, NovaPolicyRequest, NovaPolicyScope,
    NovaSceneDescriptor, NovaSceneId, NovaSceneMode, NovaSceneSwitchRequest,
    NovaServiceArtifactSpec, NovaServiceBindingState, NovaServiceBootstrapRequirement,
    NovaServiceDescriptor, NovaServiceId, NovaServiceKernelBinding, NovaServiceKernelLaunchPlan,
    NovaServiceKind, NovaServiceLaunchRequest, NovaServiceLaunchResult, NovaServiceLaunchSpec,
    NovaServiceLaunchStatus, NovaServiceState, NovaServiceStatus, NovaSharedMemoryRegionId,
    NovaStatusRequest, NovaTaskId,
};
pub use syscall::{
    NOVA_BOOTSTRAP_TRAP_IMM16, NOVA_SYSCALL_ARG_COUNT, NovaSyscallNumberV1, NovaSyscallRequestV1,
    NovaSyscallResultV1, NovaSyscallStatusV1,
};
pub use verification::NovaVerificationInfoV1;
