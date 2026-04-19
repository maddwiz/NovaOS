use crate::{NovaDigestAlgorithm, NovaImageDigestV1, sha256_digest_bytes};
use core::mem::size_of;

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct NovaPayloadHeaderV1Raw {
    magic: u64,
    version: u32,
    kind: u32,
    header_size: u32,
    image_size: u32,
    load_offset: u32,
    load_size: u32,
    entry_offset: u32,
    entry_abi: u32,
    load_mode: u32,
    flags: u32,
    body_digest_algorithm: u32,
    body_digest_len: u32,
    body_digest: [u8; 32],
}

impl NovaPayloadHeaderV1Raw {
    fn is_valid(&self) -> bool {
        self.magic == NovaPayloadHeaderV1::MAGIC
            && self.version == NovaPayloadHeaderV1::VERSION
            && self.header_size as usize == size_of::<Self>()
            && self.image_size >= self.header_size
            && self.load_mode == NovaPayloadLoadMode::FlatBinary as u32
            && self.load_offset >= self.header_size
            && self.load_size == self.image_size - self.load_offset
            && self.entry_offset >= self.load_offset
            && self.entry_offset < self.load_offset + self.load_size
            && self.flags == 0
            && self.body_digest_algorithm == NovaDigestAlgorithm::Sha256 as u32
            && self.body_digest_len as usize == self.body_digest.len()
            && NovaPayloadKind::from_raw(self.kind).is_some()
            && NovaPayloadEntryAbi::from_raw(self.entry_abi).is_some()
    }

    fn matches_image_len(&self, image_len: usize) -> bool {
        self.image_size as usize == image_len
    }

    fn decode(&self) -> Option<NovaPayloadHeaderV1> {
        Some(NovaPayloadHeaderV1 {
            magic: self.magic,
            version: self.version,
            kind: NovaPayloadKind::from_raw(self.kind)?,
            header_size: self.header_size,
            image_size: self.image_size,
            load_offset: self.load_offset,
            load_size: self.load_size,
            entry_offset: self.entry_offset,
            entry_abi: NovaPayloadEntryAbi::from_raw(self.entry_abi)?,
            load_mode: NovaPayloadLoadMode::from_raw(self.load_mode)?,
            flags: self.flags,
            body_digest_algorithm: NovaDigestAlgorithm::from_raw(self.body_digest_algorithm)?,
            body_digest_len: self.body_digest_len,
            body_digest: self.body_digest,
        })
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NovaPayloadHeaderV1 {
    pub magic: u64,
    pub version: u32,
    pub kind: NovaPayloadKind,
    pub header_size: u32,
    pub image_size: u32,
    pub load_offset: u32,
    pub load_size: u32,
    pub entry_offset: u32,
    pub entry_abi: NovaPayloadEntryAbi,
    pub load_mode: NovaPayloadLoadMode,
    pub flags: u32,
    pub body_digest_algorithm: NovaDigestAlgorithm,
    pub body_digest_len: u32,
    pub body_digest: [u8; 32],
}

impl NovaPayloadHeaderV1 {
    pub const MAGIC: u64 = 0x3159_4150_4156_4F4E;
    pub const VERSION: u32 = 1;

    pub const fn new_flat_binary(
        kind: NovaPayloadKind,
        entry_abi: NovaPayloadEntryAbi,
        image_size: u32,
        body_digest: [u8; 32],
    ) -> Self {
        let header_size = size_of::<Self>() as u32;
        Self::new_flat_binary_with_offsets(
            kind,
            entry_abi,
            image_size,
            header_size,
            header_size,
            body_digest,
        )
    }

    pub const fn new_flat_binary_with_offsets(
        kind: NovaPayloadKind,
        entry_abi: NovaPayloadEntryAbi,
        image_size: u32,
        load_offset: u32,
        entry_offset: u32,
        body_digest: [u8; 32],
    ) -> Self {
        let header_size = size_of::<Self>() as u32;
        Self {
            magic: Self::MAGIC,
            version: Self::VERSION,
            kind,
            header_size,
            image_size,
            load_offset,
            load_size: image_size - load_offset,
            entry_offset,
            entry_abi,
            load_mode: NovaPayloadLoadMode::FlatBinary,
            flags: 0,
            body_digest_algorithm: NovaDigestAlgorithm::Sha256,
            body_digest_len: 32,
            body_digest,
        }
    }

    pub const fn is_valid(&self) -> bool {
        self.magic == Self::MAGIC
            && self.version == Self::VERSION
            && self.header_size as usize == size_of::<Self>()
            && self.image_size >= self.header_size
            && self.load_mode as u32 == NovaPayloadLoadMode::FlatBinary as u32
            && self.load_offset >= self.header_size
            && self.load_size == self.image_size - self.load_offset
            && self.entry_offset >= self.load_offset
            && self.entry_offset < self.load_offset + self.load_size
            && self.body_digest_algorithm as u32 == NovaDigestAlgorithm::Sha256 as u32
            && self.body_digest_len as usize == self.body_digest.len()
            && matches!(
                self.entry_abi,
                NovaPayloadEntryAbi::Stage1Plan
                    | NovaPayloadEntryAbi::BootInfo
                    | NovaPayloadEntryAbi::BootInfoV2Sidecar
                    | NovaPayloadEntryAbi::BootstrapTaskV1
            )
    }

    pub fn matches_image_len(&self, image_len: usize) -> bool {
        self.image_size as usize == image_len
    }
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NovaPayloadKind {
    Stage1 = 1,
    Kernel = 2,
    Service = 3,
}

impl NovaPayloadKind {
    const fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            value if value == Self::Stage1 as u32 => Some(Self::Stage1),
            value if value == Self::Kernel as u32 => Some(Self::Kernel),
            value if value == Self::Service as u32 => Some(Self::Service),
            _ => None,
        }
    }
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NovaPayloadEntryAbi {
    Stage1Plan = 1,
    BootInfo = 2,
    BootInfoV2Sidecar = 3,
    BootstrapTaskV1 = 4,
}

impl NovaPayloadEntryAbi {
    const fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            value if value == Self::Stage1Plan as u32 => Some(Self::Stage1Plan),
            value if value == Self::BootInfo as u32 => Some(Self::BootInfo),
            value if value == Self::BootInfoV2Sidecar as u32 => Some(Self::BootInfoV2Sidecar),
            value if value == Self::BootstrapTaskV1 as u32 => Some(Self::BootstrapTaskV1),
            _ => None,
        }
    }
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NovaPayloadLoadMode {
    FlatBinary = 1,
}

impl NovaPayloadLoadMode {
    const fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            value if value == Self::FlatBinary as u32 => Some(Self::FlatBinary),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PayloadImage<'a> {
    header: NovaPayloadHeaderV1,
    bytes: &'a [u8],
}

impl<'a> PayloadImage<'a> {
    pub fn parse(bytes: &'a [u8]) -> Option<Self> {
        if bytes.len() < size_of::<NovaPayloadHeaderV1>() {
            return None;
        }

        let raw_header =
            unsafe { (bytes.as_ptr() as *const NovaPayloadHeaderV1Raw).read_unaligned() };
        if !raw_header.is_valid() || !raw_header.matches_image_len(bytes.len()) {
            return None;
        }
        let header = raw_header.decode()?;

        let image = Self { header, bytes };
        image.body_digest_matches().then_some(image)
    }

    pub fn parse_kind(bytes: &'a [u8], kind: NovaPayloadKind) -> Option<Self> {
        let image = Self::parse(bytes)?;
        (image.kind() == kind).then_some(image)
    }

    pub fn parse_kind_abi(
        bytes: &'a [u8],
        kind: NovaPayloadKind,
        entry_abi: NovaPayloadEntryAbi,
    ) -> Option<Self> {
        let image = Self::parse_kind(bytes, kind)?;
        (image.entry_abi() == entry_abi).then_some(image)
    }

    pub const fn header(&self) -> NovaPayloadHeaderV1 {
        self.header
    }

    pub const fn kind(&self) -> NovaPayloadKind {
        self.header.kind
    }

    pub const fn entry_abi(&self) -> NovaPayloadEntryAbi {
        self.header.entry_abi
    }

    pub const fn load_mode(&self) -> NovaPayloadLoadMode {
        self.header.load_mode
    }

    pub fn body(&self) -> &'a [u8] {
        self.load_bytes()
    }

    pub fn image_bytes(&self) -> &'a [u8] {
        self.bytes
    }

    pub fn entry_addr(&self, image_base: u64) -> u64 {
        image_base + self.header.entry_offset as u64
    }

    pub fn load_base(&self, image_base: u64) -> u64 {
        image_base + self.header.load_offset as u64
    }

    pub fn load_size(&self) -> u64 {
        self.header.load_size as u64
    }

    pub fn load_bytes(&self) -> &'a [u8] {
        let start = self.header.load_offset as usize;
        let end = start + self.header.load_size as usize;
        &self.bytes[start..end]
    }

    pub fn body_digest_matches(&self) -> bool {
        sha256_digest_bytes(self.load_bytes()) == self.header.body_digest
    }

    pub fn image_digest_matches(&self, digest: &NovaImageDigestV1) -> bool {
        digest.is_valid() && sha256_digest_bytes(self.image_bytes()) == digest.bytes
    }
}

const _: [(); 88] = [(); size_of::<NovaPayloadHeaderV1>()];
const _: [(); 88] = [(); size_of::<NovaPayloadHeaderV1Raw>()];

#[cfg(test)]
mod tests {
    use super::{
        NovaPayloadEntryAbi, NovaPayloadHeaderV1, NovaPayloadKind, NovaPayloadLoadMode,
        PayloadImage,
    };
    use crate::{NovaImageDigestV1, sha256_digest_bytes};
    use alloc::vec;
    use core::mem::size_of;

    #[test]
    fn payload_header_layout_is_stable() {
        assert_eq!(size_of::<NovaPayloadHeaderV1>(), 88);
    }

    #[test]
    fn payload_image_parses_valid_wrapped_binary() {
        let body = [1u8, 2, 3, 4];
        let header_len = size_of::<NovaPayloadHeaderV1>();
        let image_len = header_len + body.len();
        let header = NovaPayloadHeaderV1::new_flat_binary(
            NovaPayloadKind::Kernel,
            NovaPayloadEntryAbi::BootInfoV2Sidecar,
            image_len as u32,
            sha256_digest_bytes(&body),
        );
        let mut image = vec![0u8; image_len];
        image[..header_len].copy_from_slice(unsafe {
            core::slice::from_raw_parts(
                &header as *const NovaPayloadHeaderV1 as *const u8,
                size_of::<NovaPayloadHeaderV1>(),
            )
        });
        image[header_len..].copy_from_slice(&body);

        let parsed =
            PayloadImage::parse_kind(image.as_slice(), NovaPayloadKind::Kernel).expect("payload");

        assert_eq!(parsed.header(), header);
        assert_eq!(parsed.entry_abi(), NovaPayloadEntryAbi::BootInfoV2Sidecar);
        assert_eq!(parsed.load_mode(), NovaPayloadLoadMode::FlatBinary);
        assert_eq!(parsed.body(), body);
        assert_eq!(parsed.entry_addr(0x2000), 0x2000 + header_len as u64);
        assert_eq!(parsed.load_base(0x2000), 0x2000 + header_len as u64);
        assert_eq!(parsed.load_size(), 4);
    }

    #[test]
    fn payload_image_parses_padded_flat_binary() {
        let body = [5u8, 6, 7, 8];
        let header_len = size_of::<NovaPayloadHeaderV1>();
        let load_offset = 2048usize;
        let image_len = load_offset + body.len();
        let header = NovaPayloadHeaderV1::new_flat_binary_with_offsets(
            NovaPayloadKind::Kernel,
            NovaPayloadEntryAbi::BootInfoV2Sidecar,
            image_len as u32,
            load_offset as u32,
            load_offset as u32,
            sha256_digest_bytes(&body),
        );
        let mut image = vec![0u8; image_len];
        image[..header_len].copy_from_slice(unsafe {
            core::slice::from_raw_parts(
                &header as *const NovaPayloadHeaderV1 as *const u8,
                size_of::<NovaPayloadHeaderV1>(),
            )
        });
        image[load_offset..].copy_from_slice(&body);

        let parsed =
            PayloadImage::parse_kind(image.as_slice(), NovaPayloadKind::Kernel).expect("payload");

        assert_eq!(parsed.header(), header);
        assert_eq!(parsed.body(), body);
        assert_eq!(parsed.entry_addr(0x4000), 0x4000 + load_offset as u64);
        assert_eq!(parsed.load_base(0x4000), 0x4000 + load_offset as u64);
        assert_eq!(parsed.load_size(), body.len() as u64);
    }

    #[test]
    fn service_payload_parses_valid_wrapped_binary() {
        let body = [9u8, 8, 7, 6];
        let header_len = size_of::<NovaPayloadHeaderV1>();
        let image_len = header_len + body.len();
        let header = NovaPayloadHeaderV1::new_flat_binary(
            NovaPayloadKind::Service,
            NovaPayloadEntryAbi::BootstrapTaskV1,
            image_len as u32,
            sha256_digest_bytes(&body),
        );
        let mut image = vec![0u8; image_len];
        image[..header_len].copy_from_slice(unsafe {
            core::slice::from_raw_parts(
                &header as *const NovaPayloadHeaderV1 as *const u8,
                size_of::<NovaPayloadHeaderV1>(),
            )
        });
        image[header_len..].copy_from_slice(&body);

        let parsed = PayloadImage::parse_kind_abi(
            image.as_slice(),
            NovaPayloadKind::Service,
            NovaPayloadEntryAbi::BootstrapTaskV1,
        )
        .expect("service payload");

        assert_eq!(parsed.header(), header);
        assert_eq!(parsed.entry_abi(), NovaPayloadEntryAbi::BootstrapTaskV1);
        assert_eq!(parsed.kind(), NovaPayloadKind::Service);
        assert_eq!(parsed.body(), body);
    }

    #[test]
    fn payload_image_rejects_wrong_kind() {
        let body = [0u8; 4];
        let header_len = size_of::<NovaPayloadHeaderV1>();
        let image_len = header_len + body.len();
        let header = NovaPayloadHeaderV1::new_flat_binary(
            NovaPayloadKind::Stage1,
            NovaPayloadEntryAbi::Stage1Plan,
            image_len as u32,
            sha256_digest_bytes(&body),
        );
        let mut image = vec![0u8; image_len];
        image[..header_len].copy_from_slice(unsafe {
            core::slice::from_raw_parts(
                &header as *const NovaPayloadHeaderV1 as *const u8,
                size_of::<NovaPayloadHeaderV1>(),
            )
        });
        image[header_len..].copy_from_slice(&body);

        assert!(PayloadImage::parse_kind(image.as_slice(), NovaPayloadKind::Kernel).is_none());
    }

    #[test]
    fn payload_image_rejects_digest_mismatch() {
        let body = [1u8, 2, 3, 4];
        let header_len = size_of::<NovaPayloadHeaderV1>();
        let image_len = header_len + body.len();
        let header = NovaPayloadHeaderV1::new_flat_binary(
            NovaPayloadKind::Kernel,
            NovaPayloadEntryAbi::BootInfoV2Sidecar,
            image_len as u32,
            [0xFF; 32],
        );
        let mut image = vec![0u8; image_len];
        image[..header_len].copy_from_slice(unsafe {
            core::slice::from_raw_parts(
                &header as *const NovaPayloadHeaderV1 as *const u8,
                size_of::<NovaPayloadHeaderV1>(),
            )
        });
        image[header_len..].copy_from_slice(&body);

        assert!(PayloadImage::parse(image.as_slice()).is_none());
    }

    #[test]
    fn payload_image_matches_full_image_digest() {
        let body = [9u8, 8, 7, 6];
        let header_len = size_of::<NovaPayloadHeaderV1>();
        let image_len = header_len + body.len();
        let header = NovaPayloadHeaderV1::new_flat_binary(
            NovaPayloadKind::Kernel,
            NovaPayloadEntryAbi::BootInfoV2Sidecar,
            image_len as u32,
            sha256_digest_bytes(&body),
        );
        let mut image = vec![0u8; image_len];
        image[..header_len].copy_from_slice(unsafe {
            core::slice::from_raw_parts(
                &header as *const NovaPayloadHeaderV1 as *const u8,
                size_of::<NovaPayloadHeaderV1>(),
            )
        });
        image[header_len..].copy_from_slice(&body);

        let parsed = PayloadImage::parse(image.as_slice()).expect("payload");
        let digest = NovaImageDigestV1::from_bytes_sha256(image.as_slice());

        assert!(parsed.image_digest_matches(&digest));
    }

    #[test]
    fn payload_image_rejects_wrong_entry_abi() {
        let body = [3u8, 2, 1, 0];
        let header_len = size_of::<NovaPayloadHeaderV1>();
        let image_len = header_len + body.len();
        let header = NovaPayloadHeaderV1::new_flat_binary(
            NovaPayloadKind::Kernel,
            NovaPayloadEntryAbi::Stage1Plan,
            image_len as u32,
            sha256_digest_bytes(&body),
        );
        let mut image = vec![0u8; image_len];
        image[..header_len].copy_from_slice(unsafe {
            core::slice::from_raw_parts(
                &header as *const NovaPayloadHeaderV1 as *const u8,
                size_of::<NovaPayloadHeaderV1>(),
            )
        });
        image[header_len..].copy_from_slice(&body);

        assert!(
            PayloadImage::parse_kind_abi(
                image.as_slice(),
                NovaPayloadKind::Kernel,
                NovaPayloadEntryAbi::BootInfoV2Sidecar
            )
            .is_none()
        );
    }

    #[test]
    fn payload_enums_cover_service_bootstrap_contract() {
        assert_eq!(NovaPayloadKind::Stage1 as u32, 1);
        assert_eq!(NovaPayloadKind::Kernel as u32, 2);
        assert_eq!(NovaPayloadKind::Service as u32, 3);
        assert_eq!(NovaPayloadEntryAbi::Stage1Plan as u32, 1);
        assert_eq!(NovaPayloadEntryAbi::BootInfo as u32, 2);
        assert_eq!(NovaPayloadEntryAbi::BootInfoV2Sidecar as u32, 3);
        assert_eq!(NovaPayloadEntryAbi::BootstrapTaskV1 as u32, 4);
    }
}
