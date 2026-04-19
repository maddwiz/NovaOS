use core::mem::size_of;

use crate::bootinfo::{NovaBootInfoV1, NovaBootInfoV2};
use nova_rt::{
    InitCapsuleImage, NovaImageDigestV1, NovaPayloadEntryAbi, NovaPayloadKind,
    NovaVerificationInfoV1, PayloadImage,
};

pub type Stage1Entry = unsafe extern "C" fn(*const Stage1Plan) -> !;
pub type KernelEntry = unsafe extern "C" fn(*const NovaBootInfoV1, *const NovaBootInfoV2) -> !;

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
const KERNEL_BOOT_STACK_SIZE: usize = 64 * 1024;
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
static mut KERNEL_BOOT_STACK: [u8; KERNEL_BOOT_STACK_SIZE] = [0; KERNEL_BOOT_STACK_SIZE];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Stage1Input<'a> {
    pub boot_info: &'a NovaBootInfoV1,
    pub boot_info_v2: Option<&'a NovaBootInfoV2>,
    pub kernel_image: &'a [u8],
    pub init_capsule: Option<&'a [u8]>,
    pub secure_boot: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Stage1Config {
    pub allow_relocation: bool,
    pub exit_boot_services: bool,
}

impl Stage1Config {
    pub const fn strict() -> Self {
        Self {
            allow_relocation: false,
            exit_boot_services: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KernelImage {
    pub entry_point: u64,
    pub load_base: u64,
    pub load_size: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Stage1Plan {
    pub config: Stage1Config,
    pub boot_info_addr: u64,
    pub boot_info_v2_addr: u64,
    pub kernel: KernelImage,
    pub boot_info_summary: crate::bootinfo::BootSummary,
    pub boot_info_size: usize,
    pub boot_info_v2_size: usize,
    pub init_capsule_addr: u64,
    pub init_capsule_len: usize,
}

impl Stage1Plan {
    pub const fn empty() -> Self {
        Self {
            config: Stage1Config::strict(),
            boot_info_addr: 0,
            boot_info_v2_addr: 0,
            kernel: KernelImage {
                entry_point: 0,
                load_base: 0,
                load_size: 0,
            },
            boot_info_summary: crate::bootinfo::BootSummary::empty(),
            boot_info_size: 0,
            boot_info_v2_size: 0,
            init_capsule_addr: 0,
            init_capsule_len: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Stage1Status {
    Ready,
    InvalidKernelImage,
    InvalidKernelDigest,
    InvalidVerificationInfo,
    InvalidInitCapsule,
    InvalidBootInfo,
    InvalidBootInfoV2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Stage1Transfer {
    pub boot_info_addr: u64,
    pub boot_info_v2_addr: u64,
    pub kernel_entry: u64,
    pub init_capsule_addr: u64,
    pub init_capsule_len: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Stage1TransferStatus {
    InvalidPlan,
    InvalidBootInfo,
    InvalidBootInfoV2,
    MissingKernelEntry,
}

pub fn build_plan(input: &Stage1Input<'_>) -> Result<Stage1Plan, Stage1Status> {
    if !input.boot_info.is_valid() {
        return Err(Stage1Status::InvalidBootInfo);
    }

    if input
        .boot_info_v2
        .is_some_and(|boot_info_v2| !boot_info_v2.is_valid())
    {
        return Err(Stage1Status::InvalidBootInfoV2);
    }

    if input.kernel_image.is_empty() {
        return Err(Stage1Status::InvalidKernelImage);
    }

    let kernel_image = PayloadImage::parse_kind_abi(
        input.kernel_image,
        NovaPayloadKind::Kernel,
        NovaPayloadEntryAbi::BootInfoV2Sidecar,
    )
    .ok_or(Stage1Status::InvalidKernelImage)?;
    let verification =
        resolve_verification_info(input.boot_info).ok_or(Stage1Status::InvalidVerificationInfo)?;
    if !verification.stage1_payload_verified()
        || !verification.kernel_payload_verified()
        || !verification.kernel_digest_verified()
        || verification.kernel_image_size != input.kernel_image.len() as u64
    {
        return Err(Stage1Status::InvalidVerificationInfo);
    }
    let kernel_digest =
        resolve_kernel_image_digest(input.boot_info).ok_or(Stage1Status::InvalidKernelDigest)?;
    if !kernel_image.image_digest_matches(kernel_digest) {
        return Err(Stage1Status::InvalidKernelDigest);
    }
    if input
        .init_capsule
        .is_some_and(|capsule| InitCapsuleImage::parse(capsule).is_none())
    {
        return Err(Stage1Status::InvalidInitCapsule);
    }
    let kernel_base = input.kernel_image.as_ptr() as u64;
    let init_capsule_addr = input
        .init_capsule
        .map_or(0, |capsule| capsule.as_ptr() as u64);

    Ok(Stage1Plan {
        config: Stage1Config::strict(),
        boot_info_addr: input.boot_info as *const NovaBootInfoV1 as u64,
        boot_info_v2_addr: input.boot_info_v2.map_or(0, |boot_info_v2| {
            boot_info_v2 as *const NovaBootInfoV2 as u64
        }),
        kernel: KernelImage {
            entry_point: kernel_image.entry_addr(kernel_base),
            load_base: kernel_image.load_base(kernel_base),
            load_size: kernel_image.load_size(),
        },
        boot_info_summary: input.boot_info.summary(),
        boot_info_size: size_of::<NovaBootInfoV1>(),
        boot_info_v2_size: input
            .boot_info_v2
            .map_or(0, |_| size_of::<NovaBootInfoV2>()),
        init_capsule_addr,
        init_capsule_len: input.init_capsule.map_or(0, |capsule| capsule.len()),
    })
}

pub fn prepare_transfer(plan: &Stage1Plan) -> Result<Stage1Transfer, Stage1TransferStatus> {
    if plan.boot_info_addr == 0 {
        return Err(Stage1TransferStatus::InvalidPlan);
    }

    let boot_info = unsafe { &*(plan.boot_info_addr as *const NovaBootInfoV1) };
    if !boot_info.is_valid() {
        return Err(Stage1TransferStatus::InvalidBootInfo);
    }

    if plan.boot_info_v2_addr == 0 {
        if plan.boot_info_v2_size != 0 {
            return Err(Stage1TransferStatus::InvalidPlan);
        }
    } else {
        let boot_info_v2 = unsafe { (plan.boot_info_v2_addr as *const NovaBootInfoV2).as_ref() }
            .ok_or(Stage1TransferStatus::InvalidBootInfoV2)?;
        if plan.boot_info_v2_size != size_of::<NovaBootInfoV2>() || !boot_info_v2.is_valid() {
            return Err(Stage1TransferStatus::InvalidBootInfoV2);
        }
    }

    if plan.kernel.entry_point == 0 {
        return Err(Stage1TransferStatus::MissingKernelEntry);
    }

    Ok(Stage1Transfer {
        boot_info_addr: plan.boot_info_addr,
        boot_info_v2_addr: plan.boot_info_v2_addr,
        kernel_entry: plan.kernel.entry_point,
        init_capsule_addr: plan.init_capsule_addr,
        init_capsule_len: plan.init_capsule_len,
    })
}

pub fn stage1_entry(plan_ptr: *const Stage1Plan) -> ! {
    if plan_ptr.is_null() {
        halt();
    }

    let plan = unsafe { &*plan_ptr };
    let transfer = match prepare_transfer(plan) {
        Ok(transfer) => transfer,
        Err(_) => halt(),
    };

    trace_boot_info_v2_sidecar(plan);

    let kernel_entry: KernelEntry =
        unsafe { core::mem::transmute::<usize, KernelEntry>(transfer.kernel_entry as usize) };

    #[cfg(all(target_os = "none", target_arch = "aarch64"))]
    unsafe {
        enter_kernel_with_boot_stack(
            kernel_entry,
            transfer.boot_info_addr as *const NovaBootInfoV1,
            transfer.boot_info_v2_addr as *const NovaBootInfoV2,
        )
    }

    #[cfg(not(all(target_os = "none", target_arch = "aarch64")))]
    unsafe {
        kernel_entry(
            transfer.boot_info_addr as *const NovaBootInfoV1,
            transfer.boot_info_v2_addr as *const NovaBootInfoV2,
        )
    }
}

pub fn handoff(plan: &Stage1Plan) -> ! {
    stage1_entry(plan as *const Stage1Plan)
}

fn halt() -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_virt_trace"
))]
fn trace_boot_info_v2_sidecar(plan: &Stage1Plan) {
    if plan.boot_info_v2_addr != 0 && plan.boot_info_v2_size == size_of::<NovaBootInfoV2>() {
        qemu_uart_write(b"NovaOS stage1 bootinfo_v2 sidecar\n");
    } else {
        qemu_uart_write(b"NovaOS stage1 bootinfo_v2 absent\n");
    }
}

#[cfg(not(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_virt_trace"
)))]
fn trace_boot_info_v2_sidecar(_plan: &Stage1Plan) {}

#[cfg(all(
    target_os = "none",
    target_arch = "aarch64",
    feature = "qemu_virt_trace"
))]
fn qemu_uart_write(message: &[u8]) {
    const PL011_BASE: usize = 0x0900_0000;
    const PL011_DR: *mut u32 = PL011_BASE as *mut u32;
    const PL011_FR: *const u32 = (PL011_BASE + 0x18) as *const u32;
    const PL011_FR_TXFF: u32 = 1 << 5;

    for &byte in message {
        while unsafe { core::ptr::read_volatile(PL011_FR) } & PL011_FR_TXFF != 0 {}
        unsafe {
            core::ptr::write_volatile(PL011_DR, byte as u32);
        }
    }
}

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
unsafe fn enter_kernel_with_boot_stack(
    kernel_entry: KernelEntry,
    boot_info: *const NovaBootInfoV1,
    boot_info_v2: *const NovaBootInfoV2,
) -> ! {
    let stack_top = (core::ptr::addr_of_mut!(KERNEL_BOOT_STACK) as *mut u8)
        .add(KERNEL_BOOT_STACK_SIZE) as usize;
    core::arch::asm!(
        "mov sp, {stack_top}",
        "blr {kernel_entry}",
        stack_top = in(reg) stack_top,
        kernel_entry = in(reg) kernel_entry as usize,
        in("x0") boot_info,
        in("x1") boot_info_v2,
        options(noreturn),
    );
}

fn resolve_kernel_image_digest(boot_info: &NovaBootInfoV1) -> Option<&NovaImageDigestV1> {
    if !boot_info.has_flag(NovaBootInfoV1::FLAG_HAS_KERNEL_IMAGE_DIGEST) {
        return None;
    }

    let digest_ptr = boot_info.kernel_image_hash_ptr as *const NovaImageDigestV1;
    let digest = unsafe { digest_ptr.as_ref() }?;
    digest.is_valid().then_some(digest)
}

fn resolve_verification_info(boot_info: &NovaBootInfoV1) -> Option<&NovaVerificationInfoV1> {
    if !boot_info.has_flag(NovaBootInfoV1::FLAG_HAS_VERIFICATION_INFO) {
        return None;
    }

    let verification_ptr = boot_info.verification_info_ptr as *const NovaVerificationInfoV1;
    let verification = unsafe { verification_ptr.as_ref() }?;
    verification.is_valid().then_some(verification)
}

#[cfg(test)]
mod tests {
    use super::{build_plan, prepare_transfer, Stage1Input, Stage1Status, Stage1TransferStatus};
    use crate::bootinfo::{NovaBootInfoV1, NovaBootInfoV2};
    use alloc::vec::Vec;
    use nova_rt::{
        encode_init_capsule_service_name, sha256_digest_bytes, NovaImageDigestV1,
        NovaInitCapsuleCapabilityV1, NovaInitCapsuleHeaderV1, NovaPayloadEntryAbi,
        NovaPayloadHeaderV1, NovaPayloadKind, NovaVerificationInfoV1,
    };

    fn wrap_kernel_payload(body: &[u8]) -> Vec<u8> {
        let header_len = size_of::<NovaPayloadHeaderV1>();
        let image_size = header_len + body.len();
        let header = NovaPayloadHeaderV1::new_flat_binary(
            NovaPayloadKind::Kernel,
            NovaPayloadEntryAbi::BootInfoV2Sidecar,
            image_size as u32,
            sha256_digest_bytes(body),
        );
        let mut image = Vec::with_capacity(image_size);
        image.extend_from_slice(unsafe {
            core::slice::from_raw_parts(
                &header as *const NovaPayloadHeaderV1 as *const u8,
                size_of::<NovaPayloadHeaderV1>(),
            )
        });
        image.extend_from_slice(body);
        image
    }

    fn build_init_capsule() -> Vec<u8> {
        let header = NovaInitCapsuleHeaderV1::new(
            encode_init_capsule_service_name("initd").expect("service name"),
            NovaInitCapsuleCapabilityV1::BootLog as u64,
            0,
            0,
        );
        let mut image = Vec::with_capacity(size_of::<NovaInitCapsuleHeaderV1>());
        image.extend_from_slice(unsafe {
            core::slice::from_raw_parts(
                &header as *const NovaInitCapsuleHeaderV1 as *const u8,
                size_of::<NovaInitCapsuleHeaderV1>(),
            )
        });
        image
    }

    fn verification_info_for_kernel(kernel: &[u8]) -> NovaVerificationInfoV1 {
        let mut verification = NovaVerificationInfoV1::new();
        verification.stage1_image_size = 128;
        verification.kernel_image_size = kernel.len() as u64;
        verification.set_flag(NovaVerificationInfoV1::FLAG_STAGE1_PAYLOAD_PRESENT);
        verification.set_flag(NovaVerificationInfoV1::FLAG_STAGE1_PAYLOAD_VERIFIED);
        verification.set_flag(NovaVerificationInfoV1::FLAG_KERNEL_PAYLOAD_PRESENT);
        verification.set_flag(NovaVerificationInfoV1::FLAG_KERNEL_PAYLOAD_VERIFIED);
        verification.set_flag(NovaVerificationInfoV1::FLAG_KERNEL_DIGEST_PRESENT);
        verification.set_flag(NovaVerificationInfoV1::FLAG_KERNEL_DIGEST_VERIFIED);
        verification
    }

    fn boot_info_with_verification(
        digest: &NovaImageDigestV1,
        verification: &NovaVerificationInfoV1,
    ) -> NovaBootInfoV1 {
        let mut boot_info = NovaBootInfoV1::new();
        boot_info.set_flag(NovaBootInfoV1::FLAG_HAS_KERNEL_IMAGE_DIGEST);
        boot_info.set_flag(NovaBootInfoV1::FLAG_HAS_VERIFICATION_INFO);
        boot_info.kernel_image_hash_ptr = digest as *const NovaImageDigestV1 as u64;
        boot_info.verification_info_ptr = verification as *const NovaVerificationInfoV1 as u64;
        boot_info
    }

    #[test]
    fn build_plan_rejects_invalid_boot_info() {
        let kernel = wrap_kernel_payload(&[0xAA, 0x55]);
        let boot_info = NovaBootInfoV1::empty();
        let input = Stage1Input {
            boot_info: &boot_info,
            boot_info_v2: None,
            kernel_image: kernel.as_slice(),
            init_capsule: None,
            secure_boot: false,
        };

        assert_eq!(build_plan(&input), Err(Stage1Status::InvalidBootInfo));
    }

    #[test]
    fn build_plan_rejects_empty_kernel() {
        let boot_info = NovaBootInfoV1::new();
        let input = Stage1Input {
            boot_info: &boot_info,
            boot_info_v2: None,
            kernel_image: &[],
            init_capsule: None,
            secure_boot: false,
        };

        assert_eq!(build_plan(&input), Err(Stage1Status::InvalidKernelImage));
    }

    #[test]
    fn build_plan_tracks_boot_info_and_payload_locations() {
        let kernel = wrap_kernel_payload(&[1u8, 2, 3, 4]);
        let digest = NovaImageDigestV1::from_bytes_sha256(kernel.as_slice());
        let verification = verification_info_for_kernel(kernel.as_slice());
        let boot_info = boot_info_with_verification(&digest, &verification);
        let boot_info_v2 = NovaBootInfoV2::new();
        let init_capsule = build_init_capsule();
        let input = Stage1Input {
            boot_info: &boot_info,
            boot_info_v2: Some(&boot_info_v2),
            kernel_image: kernel.as_slice(),
            init_capsule: Some(init_capsule.as_slice()),
            secure_boot: false,
        };

        let plan = build_plan(&input).expect("stage1 plan");
        let entry_offset = size_of::<NovaPayloadHeaderV1>() as u64;

        assert_eq!(
            plan.boot_info_addr,
            &boot_info as *const NovaBootInfoV1 as u64
        );
        assert_eq!(
            plan.boot_info_v2_addr,
            &boot_info_v2 as *const NovaBootInfoV2 as u64
        );
        assert_eq!(plan.boot_info_v2_size, size_of::<NovaBootInfoV2>());
        assert_eq!(plan.kernel.load_base, kernel.as_ptr() as u64 + entry_offset);
        assert_eq!(
            plan.kernel.entry_point,
            kernel.as_ptr() as u64 + entry_offset
        );
        assert_eq!(plan.kernel.load_size, 4);
        assert_eq!(plan.init_capsule_addr, init_capsule.as_ptr() as u64);
        assert_eq!(plan.init_capsule_len, init_capsule.len());
    }

    #[test]
    fn build_plan_rejects_invalid_init_capsule() {
        let kernel = wrap_kernel_payload(&[1u8, 2, 3, 4]);
        let digest = NovaImageDigestV1::from_bytes_sha256(kernel.as_slice());
        let verification = verification_info_for_kernel(kernel.as_slice());
        let boot_info = boot_info_with_verification(&digest, &verification);
        let invalid_capsule = [1u8, 2, 3, 4];

        let input = Stage1Input {
            boot_info: &boot_info,
            boot_info_v2: None,
            kernel_image: kernel.as_slice(),
            init_capsule: Some(&invalid_capsule),
            secure_boot: false,
        };

        assert_eq!(build_plan(&input), Err(Stage1Status::InvalidInitCapsule));
    }

    #[test]
    fn prepare_transfer_rejects_invalid_boot_info_pointer() {
        let invalid_boot_info = NovaBootInfoV1::empty();
        let plan = crate::handoff::Stage1Plan {
            config: crate::handoff::Stage1Config::strict(),
            boot_info_addr: &invalid_boot_info as *const NovaBootInfoV1 as u64,
            boot_info_v2_addr: 0,
            kernel: crate::handoff::KernelImage {
                entry_point: 0x1000,
                load_base: 0x1000,
                load_size: 0x40,
            },
            boot_info_summary: invalid_boot_info.summary(),
            boot_info_size: size_of::<NovaBootInfoV1>(),
            boot_info_v2_size: 0,
            init_capsule_addr: 0,
            init_capsule_len: 0,
        };

        assert_eq!(
            prepare_transfer(&plan),
            Err(Stage1TransferStatus::InvalidBootInfo)
        );
    }

    #[test]
    fn prepare_transfer_returns_kernel_entry_and_boot_info() {
        let kernel = wrap_kernel_payload(&[1u8, 2, 3, 4]);
        let digest = NovaImageDigestV1::from_bytes_sha256(kernel.as_slice());
        let verification = verification_info_for_kernel(kernel.as_slice());
        let boot_info = boot_info_with_verification(&digest, &verification);
        let plan = build_plan(&Stage1Input {
            boot_info: &boot_info,
            boot_info_v2: None,
            kernel_image: kernel.as_slice(),
            init_capsule: None,
            secure_boot: false,
        })
        .expect("stage1 plan");

        let transfer = prepare_transfer(&plan).expect("stage1 transfer");

        assert_eq!(transfer.boot_info_addr, plan.boot_info_addr);
        assert_eq!(transfer.boot_info_v2_addr, 0);
        assert_eq!(transfer.kernel_entry, plan.kernel.entry_point);
    }

    #[test]
    fn prepare_transfer_carries_boot_info_v2_pointer() {
        let kernel = wrap_kernel_payload(&[1u8, 2, 3, 4]);
        let digest = NovaImageDigestV1::from_bytes_sha256(kernel.as_slice());
        let verification = verification_info_for_kernel(kernel.as_slice());
        let boot_info = boot_info_with_verification(&digest, &verification);
        let boot_info_v2 = NovaBootInfoV2::new();
        let plan = build_plan(&Stage1Input {
            boot_info: &boot_info,
            boot_info_v2: Some(&boot_info_v2),
            kernel_image: kernel.as_slice(),
            init_capsule: None,
            secure_boot: false,
        })
        .expect("stage1 plan");

        let transfer = prepare_transfer(&plan).expect("stage1 transfer");

        assert_eq!(transfer.boot_info_addr, plan.boot_info_addr);
        assert_eq!(transfer.boot_info_v2_addr, plan.boot_info_v2_addr);
        assert_eq!(transfer.kernel_entry, plan.kernel.entry_point);
    }

    #[test]
    fn build_plan_rejects_kernel_digest_mismatch() {
        let kernel = wrap_kernel_payload(&[1u8, 2, 3, 4]);
        let verification = verification_info_for_kernel(kernel.as_slice());
        let mut boot_info = boot_info_with_verification(
            &NovaImageDigestV1::from_bytes_sha256(kernel.as_slice()),
            &verification,
        );
        let digest = NovaImageDigestV1::from_bytes_sha256(b"wrong");
        boot_info.set_flag(NovaBootInfoV1::FLAG_HAS_KERNEL_IMAGE_DIGEST);
        boot_info.kernel_image_hash_ptr = &digest as *const NovaImageDigestV1 as u64;

        let input = Stage1Input {
            boot_info: &boot_info,
            boot_info_v2: None,
            kernel_image: kernel.as_slice(),
            init_capsule: None,
            secure_boot: false,
        };

        assert_eq!(build_plan(&input), Err(Stage1Status::InvalidKernelDigest));
    }

    #[test]
    fn build_plan_rejects_missing_verification_info() {
        let kernel = wrap_kernel_payload(&[0x12u8, 0x34, 0x56]);
        let digest = NovaImageDigestV1::from_bytes_sha256(kernel.as_slice());
        let mut boot_info = NovaBootInfoV1::new();
        boot_info.set_flag(NovaBootInfoV1::FLAG_HAS_KERNEL_IMAGE_DIGEST);
        boot_info.kernel_image_hash_ptr = &digest as *const NovaImageDigestV1 as u64;

        let input = Stage1Input {
            boot_info: &boot_info,
            boot_info_v2: None,
            kernel_image: kernel.as_slice(),
            init_capsule: None,
            secure_boot: false,
        };

        assert_eq!(
            build_plan(&input),
            Err(Stage1Status::InvalidVerificationInfo)
        );
    }

    #[test]
    fn build_plan_rejects_invalid_boot_info_v2() {
        let kernel = wrap_kernel_payload(&[0xAA, 0x55]);
        let boot_info = NovaBootInfoV1::new();
        let boot_info_v2 = NovaBootInfoV2::empty();
        let input = Stage1Input {
            boot_info: &boot_info,
            boot_info_v2: Some(&boot_info_v2),
            kernel_image: kernel.as_slice(),
            init_capsule: None,
            secure_boot: false,
        };

        assert_eq!(build_plan(&input), Err(Stage1Status::InvalidBootInfoV2));
    }

    #[test]
    fn prepare_transfer_rejects_invalid_boot_info_v2_sidecar() {
        let kernel = wrap_kernel_payload(&[1u8, 2, 3, 4]);
        let digest = NovaImageDigestV1::from_bytes_sha256(kernel.as_slice());
        let verification = verification_info_for_kernel(kernel.as_slice());
        let boot_info = boot_info_with_verification(&digest, &verification);
        let mut plan = build_plan(&Stage1Input {
            boot_info: &boot_info,
            boot_info_v2: None,
            kernel_image: kernel.as_slice(),
            init_capsule: None,
            secure_boot: false,
        })
        .expect("stage1 plan");

        let invalid_boot_info_v2 = NovaBootInfoV2::empty();
        plan.boot_info_v2_addr = &invalid_boot_info_v2 as *const NovaBootInfoV2 as u64;
        plan.boot_info_v2_size = size_of::<NovaBootInfoV2>();

        assert_eq!(
            prepare_transfer(&plan),
            Err(Stage1TransferStatus::InvalidBootInfoV2)
        );
    }
}
