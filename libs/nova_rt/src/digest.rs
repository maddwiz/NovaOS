use core::mem::size_of;
use sha2::{Digest, Sha256};

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NovaImageDigestV1 {
    pub magic: u64,
    pub algorithm: NovaDigestAlgorithm,
    pub byte_len: u32,
    pub bytes: [u8; 32],
}

impl NovaImageDigestV1 {
    pub const MAGIC: u64 = 0x3154_5347_444D_564E;

    pub const fn sha256(bytes: [u8; 32]) -> Self {
        Self {
            magic: Self::MAGIC,
            algorithm: NovaDigestAlgorithm::Sha256,
            byte_len: 32,
            bytes,
        }
    }

    pub fn from_bytes_sha256(bytes: &[u8]) -> Self {
        Self::sha256(sha256_digest_bytes(bytes))
    }

    pub const fn is_valid(&self) -> bool {
        self.magic == Self::MAGIC
            && self.algorithm as u32 == NovaDigestAlgorithm::Sha256 as u32
            && self.byte_len as usize == self.bytes.len()
    }
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NovaDigestAlgorithm {
    Sha256 = 1,
}

impl NovaDigestAlgorithm {
    pub const fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            value if value == Self::Sha256 as u32 => Some(Self::Sha256),
            _ => None,
        }
    }
}

pub fn sha256_digest_bytes(bytes: &[u8]) -> [u8; 32] {
    let digest = Sha256::digest(bytes);
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

const _: [(); 48] = [(); size_of::<NovaImageDigestV1>()];

#[cfg(test)]
mod tests {
    use super::{NovaDigestAlgorithm, NovaImageDigestV1, sha256_digest_bytes};
    use core::mem::size_of;

    #[test]
    fn image_digest_layout_is_stable() {
        assert_eq!(size_of::<NovaImageDigestV1>(), 48);
    }

    #[test]
    fn sha256_digest_is_valid() {
        let digest = NovaImageDigestV1::sha256([0xA5; 32]);
        assert!(digest.is_valid());
        assert_eq!(digest.algorithm, NovaDigestAlgorithm::Sha256);
        assert_eq!(digest.byte_len, 32);
    }

    #[test]
    fn sha256_digest_bytes_matches_object_helper() {
        let bytes = sha256_digest_bytes(b"nova");
        let digest = NovaImageDigestV1::from_bytes_sha256(b"nova");
        assert_eq!(digest.bytes, bytes);
    }
}
