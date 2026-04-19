use core::mem::size_of;

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NovaVerificationInfoV1 {
    pub magic: u64,
    pub version: u32,
    pub flags: u32,
    pub stage1_image_size: u64,
    pub kernel_image_size: u64,
}

impl NovaVerificationInfoV1 {
    pub const MAGIC: u64 = 0x3146_4952_4556_4F4E;
    pub const VERSION: u32 = 1;

    pub const FLAG_STAGE1_PAYLOAD_PRESENT: u32 = 1 << 0;
    pub const FLAG_STAGE1_PAYLOAD_VERIFIED: u32 = 1 << 1;
    pub const FLAG_KERNEL_PAYLOAD_PRESENT: u32 = 1 << 2;
    pub const FLAG_KERNEL_PAYLOAD_VERIFIED: u32 = 1 << 3;
    pub const FLAG_KERNEL_DIGEST_PRESENT: u32 = 1 << 4;
    pub const FLAG_KERNEL_DIGEST_VERIFIED: u32 = 1 << 5;
    pub const FLAG_INIT_CAPSULE_PRESENT: u32 = 1 << 6;

    pub const ZERO: Self = Self {
        magic: 0,
        version: 0,
        flags: 0,
        stage1_image_size: 0,
        kernel_image_size: 0,
    };

    pub const fn new() -> Self {
        Self {
            magic: Self::MAGIC,
            version: Self::VERSION,
            ..Self::ZERO
        }
    }

    pub const fn has_flag(&self, flag: u32) -> bool {
        (self.flags & flag) != 0
    }

    pub fn set_flag(&mut self, flag: u32) {
        self.flags |= flag;
    }

    pub const fn is_valid(&self) -> bool {
        self.magic == Self::MAGIC
            && self.version == Self::VERSION
            && (!self.has_flag(Self::FLAG_STAGE1_PAYLOAD_PRESENT) || self.stage1_image_size != 0)
            && (!self.has_flag(Self::FLAG_KERNEL_PAYLOAD_PRESENT) || self.kernel_image_size != 0)
            && (!self.has_flag(Self::FLAG_STAGE1_PAYLOAD_VERIFIED)
                || self.has_flag(Self::FLAG_STAGE1_PAYLOAD_PRESENT))
            && (!self.has_flag(Self::FLAG_KERNEL_PAYLOAD_VERIFIED)
                || self.has_flag(Self::FLAG_KERNEL_PAYLOAD_PRESENT))
            && (!self.has_flag(Self::FLAG_KERNEL_DIGEST_VERIFIED)
                || self.has_flag(Self::FLAG_KERNEL_DIGEST_PRESENT))
    }

    pub const fn stage1_payload_verified(&self) -> bool {
        self.has_flag(Self::FLAG_STAGE1_PAYLOAD_VERIFIED)
    }

    pub const fn kernel_payload_verified(&self) -> bool {
        self.has_flag(Self::FLAG_KERNEL_PAYLOAD_VERIFIED)
    }

    pub const fn kernel_digest_verified(&self) -> bool {
        self.has_flag(Self::FLAG_KERNEL_DIGEST_VERIFIED)
    }
}

const _: [(); 32] = [(); size_of::<NovaVerificationInfoV1>()];

#[cfg(test)]
mod tests {
    use super::NovaVerificationInfoV1;
    use core::mem::size_of;

    #[test]
    fn verification_info_layout_is_stable() {
        assert_eq!(size_of::<NovaVerificationInfoV1>(), 32);
    }

    #[test]
    fn verification_info_requires_consistent_flags() {
        let mut info = NovaVerificationInfoV1::new();
        info.set_flag(NovaVerificationInfoV1::FLAG_KERNEL_DIGEST_VERIFIED);
        assert!(!info.is_valid());

        let mut info = NovaVerificationInfoV1::new();
        info.stage1_image_size = 64;
        info.kernel_image_size = 128;
        info.set_flag(NovaVerificationInfoV1::FLAG_STAGE1_PAYLOAD_PRESENT);
        info.set_flag(NovaVerificationInfoV1::FLAG_STAGE1_PAYLOAD_VERIFIED);
        info.set_flag(NovaVerificationInfoV1::FLAG_KERNEL_PAYLOAD_PRESENT);
        info.set_flag(NovaVerificationInfoV1::FLAG_KERNEL_PAYLOAD_VERIFIED);
        info.set_flag(NovaVerificationInfoV1::FLAG_KERNEL_DIGEST_PRESENT);
        info.set_flag(NovaVerificationInfoV1::FLAG_KERNEL_DIGEST_VERIFIED);
        assert!(info.is_valid());
    }
}
