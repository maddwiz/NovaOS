use core::mem::size_of;

use crate::syscall::trace_request;
use crate::{
    NOVA_INIT_CAPSULE_SERVICE_NAME_LEN, NovaSyscallRequestV1, NovaSyscallResultV1,
    decode_init_capsule_service_name,
};

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NovaBootstrapTaskContextV1 {
    pub magic: u64,
    pub version: u32,
    pub size: u32,
    pub service_name: [u8; NOVA_INIT_CAPSULE_SERVICE_NAME_LEN],
    pub requested_capabilities: u64,
    pub endpoint_slots: u32,
    pub shared_memory_regions: u32,
}

pub type NovaBootstrapKernelCallEntryV1 = unsafe extern "C" fn(
    *const NovaBootstrapTaskContextV2,
    *const NovaSyscallRequestV1,
) -> NovaSyscallResultV1;

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NovaBootstrapTaskContextV2 {
    pub magic: u64,
    pub version: u32,
    pub size: u32,
    pub service_name: [u8; NOVA_INIT_CAPSULE_SERVICE_NAME_LEN],
    pub requested_capabilities: u64,
    pub endpoint_slots: u32,
    pub shared_memory_regions: u32,
    pub kernel_call_entry: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResolvedBootstrapTaskContext {
    V1(&'static NovaBootstrapTaskContextV1),
    V2(&'static NovaBootstrapTaskContextV2),
}

impl NovaBootstrapTaskContextV1 {
    pub const MAGIC: u64 = 0x3158_5443_5453_424e;
    pub const VERSION: u32 = 1;

    pub const fn empty() -> Self {
        Self {
            magic: 0,
            version: 0,
            size: 0,
            service_name: [0; NOVA_INIT_CAPSULE_SERVICE_NAME_LEN],
            requested_capabilities: 0,
            endpoint_slots: 0,
            shared_memory_regions: 0,
        }
    }

    pub const fn new(
        service_name: [u8; NOVA_INIT_CAPSULE_SERVICE_NAME_LEN],
        requested_capabilities: u64,
        endpoint_slots: u32,
        shared_memory_regions: u32,
    ) -> Self {
        Self {
            magic: Self::MAGIC,
            version: Self::VERSION,
            size: size_of::<Self>() as u32,
            service_name,
            requested_capabilities,
            endpoint_slots,
            shared_memory_regions,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.magic == Self::MAGIC
            && self.version == Self::VERSION
            && self.size as usize == size_of::<Self>()
            && decode_init_capsule_service_name(&self.service_name).is_some()
    }

    pub fn service_name(&self) -> &str {
        decode_init_capsule_service_name(&self.service_name)
            .expect("bootstrap task context service name must already be valid")
    }
}

impl NovaBootstrapTaskContextV2 {
    pub const MAGIC: u64 = NovaBootstrapTaskContextV1::MAGIC;
    pub const VERSION: u32 = 2;

    pub const fn empty() -> Self {
        Self {
            magic: 0,
            version: 0,
            size: 0,
            service_name: [0; NOVA_INIT_CAPSULE_SERVICE_NAME_LEN],
            requested_capabilities: 0,
            endpoint_slots: 0,
            shared_memory_regions: 0,
            kernel_call_entry: 0,
        }
    }

    pub const fn new(
        service_name: [u8; NOVA_INIT_CAPSULE_SERVICE_NAME_LEN],
        requested_capabilities: u64,
        endpoint_slots: u32,
        shared_memory_regions: u32,
        kernel_call_entry: u64,
    ) -> Self {
        Self {
            magic: Self::MAGIC,
            version: Self::VERSION,
            size: size_of::<Self>() as u32,
            service_name,
            requested_capabilities,
            endpoint_slots,
            shared_memory_regions,
            kernel_call_entry,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.magic == Self::MAGIC
            && self.version == Self::VERSION
            && self.size as usize == size_of::<Self>()
            && self.kernel_call_entry != 0
            && decode_init_capsule_service_name(&self.service_name).is_some()
    }

    pub fn service_name(&self) -> &str {
        decode_init_capsule_service_name(&self.service_name)
            .expect("bootstrap task context service name must already be valid")
    }

    pub fn kernel_call_entry(&self) -> NovaBootstrapKernelCallEntryV1 {
        unsafe {
            core::mem::transmute::<usize, NovaBootstrapKernelCallEntryV1>(
                self.kernel_call_entry as usize,
            )
        }
    }
}

impl ResolvedBootstrapTaskContext {
    pub fn service_name(self) -> &'static str {
        match self {
            Self::V1(context) => context.service_name(),
            Self::V2(context) => context.service_name(),
        }
    }

    pub const fn requested_capabilities(self) -> u64 {
        match self {
            Self::V1(context) => context.requested_capabilities,
            Self::V2(context) => context.requested_capabilities,
        }
    }

    pub const fn endpoint_slots(self) -> u32 {
        match self {
            Self::V1(context) => context.endpoint_slots,
            Self::V2(context) => context.endpoint_slots,
        }
    }

    pub const fn shared_memory_regions(self) -> u32 {
        match self {
            Self::V1(context) => context.shared_memory_regions,
            Self::V2(context) => context.shared_memory_regions,
        }
    }

    pub fn as_v2_ptr(self) -> Option<*const NovaBootstrapTaskContextV2> {
        match self {
            Self::V1(_) => None,
            Self::V2(context) => Some(context as *const NovaBootstrapTaskContextV2),
        }
    }

    pub fn kernel_call_entry(self) -> Option<NovaBootstrapKernelCallEntryV1> {
        match self {
            Self::V1(_) => None,
            Self::V2(context) => Some(context.kernel_call_entry()),
        }
    }
}

pub fn resolve_bootstrap_task_context(
    context: *const NovaBootstrapTaskContextV1,
) -> Option<ResolvedBootstrapTaskContext> {
    let context_ptr = context;
    let context = unsafe { context_ptr.as_ref() }?;

    match context.version {
        NovaBootstrapTaskContextV1::VERSION => context
            .is_valid()
            .then_some(ResolvedBootstrapTaskContext::V1(context)),
        NovaBootstrapTaskContextV2::VERSION => {
            let context = unsafe { (context_ptr as *const NovaBootstrapTaskContextV2).as_ref() }?;
            context
                .is_valid()
                .then_some(ResolvedBootstrapTaskContext::V2(context))
        }
        _ => None,
    }
}

pub fn bootstrap_kernel_call(
    context: *const NovaBootstrapTaskContextV1,
    request: NovaSyscallRequestV1,
) -> NovaSyscallResultV1 {
    let Some(context) = resolve_bootstrap_task_context(context) else {
        return NovaSyscallResultV1::invalid_args();
    };
    let Some(context_ptr) = context.as_v2_ptr() else {
        return NovaSyscallResultV1::unsupported();
    };
    let Some(entry) = context.kernel_call_entry() else {
        return NovaSyscallResultV1::unsupported();
    };

    unsafe { entry(context_ptr, &request as *const NovaSyscallRequestV1) }
}

pub fn bootstrap_trace(
    context: *const NovaBootstrapTaskContextV1,
    value0: u64,
    value1: u64,
) -> NovaSyscallResultV1 {
    bootstrap_kernel_call(context, trace_request(value0, value1))
}

const _: [(); 48] = [(); size_of::<NovaBootstrapTaskContextV1>()];
const _: [(); 56] = [(); size_of::<NovaBootstrapTaskContextV2>()];

#[cfg(test)]
mod tests {
    use super::{
        NovaBootstrapTaskContextV1, NovaBootstrapTaskContextV2, bootstrap_trace,
        resolve_bootstrap_task_context,
    };
    use crate::{
        NovaSyscallRequestV1, NovaSyscallResultV1, NovaSyscallStatusV1,
        encode_init_capsule_service_name,
    };
    use core::mem::{offset_of, size_of};

    #[test]
    fn bootstrap_task_context_layout_is_stable() {
        assert_eq!(size_of::<NovaBootstrapTaskContextV1>(), 48);
        assert_eq!(offset_of!(NovaBootstrapTaskContextV1, service_name), 16);
        assert_eq!(size_of::<NovaBootstrapTaskContextV2>(), 56);
        assert_eq!(
            offset_of!(NovaBootstrapTaskContextV2, kernel_call_entry),
            48
        );
    }

    #[test]
    fn bootstrap_task_context_requires_valid_service_name() {
        let context = NovaBootstrapTaskContextV1::new(
            encode_init_capsule_service_name("initd").expect("service name"),
            1,
            2,
            3,
        );

        assert!(context.is_valid());
        assert_eq!(context.service_name(), "initd");
        assert_eq!(
            resolve_bootstrap_task_context(&context as *const NovaBootstrapTaskContextV1)
                .expect("context")
                .endpoint_slots(),
            2
        );
    }

    #[test]
    fn bootstrap_task_context_rejects_invalid_marker() {
        let context = NovaBootstrapTaskContextV1::empty();

        assert!(
            resolve_bootstrap_task_context(&context as *const NovaBootstrapTaskContextV1).is_none()
        );
    }

    #[test]
    fn bootstrap_task_context_v2_exposes_kernel_call_gate() {
        let context = NovaBootstrapTaskContextV2::new(
            encode_init_capsule_service_name("initd").expect("service name"),
            1,
            2,
            3,
            bootstrap_test_kernel_call as *const () as usize as u64,
        );

        let resolved = resolve_bootstrap_task_context(
            &context as *const NovaBootstrapTaskContextV2 as *const _,
        )
        .expect("context");
        assert_eq!(resolved.service_name(), "initd");
        assert_eq!(resolved.requested_capabilities(), 1);
        assert_eq!(resolved.endpoint_slots(), 2);
        assert_eq!(resolved.shared_memory_regions(), 3);
        assert!(resolved.kernel_call_entry().is_some());
    }

    #[test]
    fn bootstrap_trace_round_trips_through_context_call_gate() {
        let context = NovaBootstrapTaskContextV2::new(
            encode_init_capsule_service_name("initd").expect("service name"),
            1,
            1,
            0,
            bootstrap_test_kernel_call as *const () as usize as u64,
        );

        let result = bootstrap_trace(
            &context as *const NovaBootstrapTaskContextV2 as *const NovaBootstrapTaskContextV1,
            0xCAFE_BABE,
            0x5151_0001,
        );

        assert_eq!(result.status, NovaSyscallStatusV1::Ok as u32);
        assert_eq!(result.value0, 0xCAFE_BABE);
        assert_eq!(result.value1, 0x5151_0001);
    }

    #[test]
    fn bootstrap_trace_requires_v2_call_gate() {
        let context = NovaBootstrapTaskContextV1::new(
            encode_init_capsule_service_name("initd").expect("service name"),
            1,
            1,
            0,
        );

        let result = bootstrap_trace(&context as *const NovaBootstrapTaskContextV1, 1, 2);

        assert_eq!(result.status, NovaSyscallStatusV1::Unsupported as u32);
    }

    unsafe extern "C" fn bootstrap_test_kernel_call(
        context: *const NovaBootstrapTaskContextV2,
        request: *const NovaSyscallRequestV1,
    ) -> NovaSyscallResultV1 {
        let context = unsafe { context.as_ref() }.expect("context");
        let request = unsafe { request.as_ref() }.expect("request");

        assert_eq!(context.service_name(), "initd");
        NovaSyscallResultV1::ok(request.args[0], request.args[1])
    }
}
