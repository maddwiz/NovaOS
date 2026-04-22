use crate::{AccelBackend, BackendDescriptor, describe_backend};
use nova_fabric::{AccelSeedV1, PlatformClass, QueueClass};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AccelDispatchRequest {
    pub queue_class: QueueClass,
    pub allow_cpu_fallback: bool,
}

impl AccelDispatchRequest {
    pub const fn new(queue_class: QueueClass, allow_cpu_fallback: bool) -> Self {
        Self {
            queue_class,
            allow_cpu_fallback,
        }
    }

    pub const fn exact(queue_class: QueueClass) -> Self {
        Self::new(queue_class, false)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum AccelDispatchStatus {
    Ready = 1,
    CpuFallback = 2,
    MissingPlatformSeed = 3,
    UnsupportedQueue = 4,
    NoBackend = 5,
}

impl AccelDispatchStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::CpuFallback => "cpu-fallback",
            Self::MissingPlatformSeed => "missing-platform-seed",
            Self::UnsupportedQueue => "unsupported-queue",
            Self::NoBackend => "no-backend",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AccelDispatchPlan {
    pub request: AccelDispatchRequest,
    pub seed_ready: bool,
    pub selected_backend: Option<BackendDescriptor>,
    pub status: AccelDispatchStatus,
}

impl AccelDispatchPlan {
    pub const fn new(
        request: AccelDispatchRequest,
        seed_ready: bool,
        selected_backend: Option<BackendDescriptor>,
        status: AccelDispatchStatus,
    ) -> Self {
        Self {
            request,
            seed_ready,
            selected_backend,
            status,
        }
    }

    pub const fn is_ready(self) -> bool {
        matches!(
            self.status,
            AccelDispatchStatus::Ready | AccelDispatchStatus::CpuFallback
        )
    }

    pub const fn used_cpu_fallback(self) -> bool {
        matches!(self.status, AccelDispatchStatus::CpuFallback)
    }
}

pub fn plan_accel_dispatch(
    backends: &[&dyn AccelBackend],
    seed: &AccelSeedV1,
    request: AccelDispatchRequest,
) -> AccelDispatchPlan {
    let seed_ready = seed.platform_ready();
    let mut saw_matching_backend = false;

    if seed_ready {
        for backend in backends {
            if backend.supports_seed(seed) {
                saw_matching_backend = true;
                if queue_supported(*backend, request.queue_class) {
                    return AccelDispatchPlan::new(
                        request,
                        seed_ready,
                        Some(describe_backend(*backend)),
                        AccelDispatchStatus::Ready,
                    );
                }
            }
        }
    }

    if request.allow_cpu_fallback {
        for backend in backends {
            if backend.platform_class() == PlatformClass::Unknown
                && queue_supported(*backend, request.queue_class)
            {
                return AccelDispatchPlan::new(
                    request,
                    seed_ready,
                    Some(describe_backend(*backend)),
                    AccelDispatchStatus::CpuFallback,
                );
            }
        }
    }

    if !seed_ready {
        return AccelDispatchPlan::new(
            request,
            seed_ready,
            None,
            AccelDispatchStatus::MissingPlatformSeed,
        );
    }

    if saw_matching_backend {
        return AccelDispatchPlan::new(
            request,
            seed_ready,
            None,
            AccelDispatchStatus::UnsupportedQueue,
        );
    }

    AccelDispatchPlan::new(request, seed_ready, None, AccelDispatchStatus::NoBackend)
}

fn queue_supported(backend: &dyn AccelBackend, queue_class: QueueClass) -> bool {
    backend.supported_queue_classes().contains(&queue_class)
}
