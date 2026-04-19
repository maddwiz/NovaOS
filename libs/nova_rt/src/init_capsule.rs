use core::mem::size_of;

use crate::{NovaPayloadEntryAbi, NovaPayloadKind, PayloadImage};

pub const NOVA_INIT_CAPSULE_SERVICE_NAME_LEN: usize = 16;
pub const NOVA_INIT_CAPSULE_KNOWN_CAPABILITIES_V1: u64 = NovaInitCapsuleCapabilityV1::BootLog
    as u64
    | NovaInitCapsuleCapabilityV1::Yield as u64
    | NovaInitCapsuleCapabilityV1::EndpointBootstrap as u64
    | NovaInitCapsuleCapabilityV1::SharedMemoryBootstrap as u64;

#[repr(u64)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NovaInitCapsuleCapabilityV1 {
    BootLog = 1 << 0,
    Yield = 1 << 1,
    EndpointBootstrap = 1 << 2,
    SharedMemoryBootstrap = 1 << 3,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NovaInitCapsuleHeaderV1 {
    pub magic: u64,
    pub version: u32,
    pub header_size: u32,
    pub total_size: u32,
    pub flags: u32,
    pub requested_capabilities: u64,
    pub endpoint_slots: u32,
    pub shared_memory_regions: u32,
    pub service_name: [u8; NOVA_INIT_CAPSULE_SERVICE_NAME_LEN],
    pub reserved: [u8; 8],
}

impl NovaInitCapsuleHeaderV1 {
    pub const MAGIC: u64 = 0x5449_4E49_4156_4F4E;
    pub const VERSION: u32 = 1;

    pub const fn new(
        service_name: [u8; NOVA_INIT_CAPSULE_SERVICE_NAME_LEN],
        requested_capabilities: u64,
        endpoint_slots: u32,
        shared_memory_regions: u32,
    ) -> Self {
        Self {
            magic: Self::MAGIC,
            version: Self::VERSION,
            header_size: size_of::<Self>() as u32,
            total_size: size_of::<Self>() as u32,
            flags: 0,
            requested_capabilities,
            endpoint_slots,
            shared_memory_regions,
            service_name,
            reserved: [0; 8],
        }
    }

    pub fn is_valid(&self) -> bool {
        self.magic == Self::MAGIC
            && self.version == Self::VERSION
            && self.header_size as usize == size_of::<Self>()
            && self.total_size >= self.header_size
            && self.flags == 0
            && (self.requested_capabilities & !NOVA_INIT_CAPSULE_KNOWN_CAPABILITIES_V1) == 0
            && decode_init_capsule_service_name(&self.service_name).is_some()
    }

    pub fn matches_image_len(&self, image_len: usize) -> bool {
        self.total_size as usize == image_len
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InitCapsuleImage<'a> {
    header: NovaInitCapsuleHeaderV1,
    bytes: &'a [u8],
}

impl<'a> InitCapsuleImage<'a> {
    pub fn parse(bytes: &'a [u8]) -> Option<Self> {
        if bytes.len() < size_of::<NovaInitCapsuleHeaderV1>() {
            return None;
        }

        let header = unsafe { (bytes.as_ptr() as *const NovaInitCapsuleHeaderV1).read_unaligned() };
        if !header.is_valid() || !header.matches_image_len(bytes.len()) {
            return None;
        }

        let image = Self { header, bytes };
        image.has_valid_body().then_some(image)
    }

    pub const fn header(&self) -> NovaInitCapsuleHeaderV1 {
        self.header
    }

    pub fn service_name(&self) -> &str {
        decode_init_capsule_service_name(&self.header.service_name)
            .expect("init capsule service name must already be valid")
    }

    pub const fn requested_capabilities(&self) -> u64 {
        self.header.requested_capabilities
    }

    pub const fn endpoint_slots(&self) -> u32 {
        self.header.endpoint_slots
    }

    pub const fn shared_memory_regions(&self) -> u32 {
        self.header.shared_memory_regions
    }

    pub fn body(&self) -> &'a [u8] {
        &self.bytes[self.header.header_size as usize..]
    }

    pub fn bootstrap_service_payload(&self) -> Option<PayloadImage<'a>> {
        let body = self.body();
        if body.is_empty() {
            return None;
        }

        PayloadImage::parse_kind_abi(
            body,
            NovaPayloadKind::Service,
            NovaPayloadEntryAbi::BootstrapTaskV1,
        )
    }

    fn has_valid_body(&self) -> bool {
        self.body().is_empty() || self.bootstrap_service_payload().is_some()
    }
}

pub fn encode_init_capsule_service_name(
    name: &str,
) -> Option<[u8; NOVA_INIT_CAPSULE_SERVICE_NAME_LEN]> {
    if name.is_empty() || name.len() > NOVA_INIT_CAPSULE_SERVICE_NAME_LEN {
        return None;
    }

    let mut encoded = [0u8; NOVA_INIT_CAPSULE_SERVICE_NAME_LEN];
    for (index, byte) in name.bytes().enumerate() {
        if !is_valid_service_name_byte(byte) {
            return None;
        }
        encoded[index] = byte;
    }

    Some(encoded)
}

pub fn decode_init_capsule_service_name(
    bytes: &[u8; NOVA_INIT_CAPSULE_SERVICE_NAME_LEN],
) -> Option<&str> {
    let mut len = 0usize;
    let mut saw_terminator = false;

    while len < bytes.len() {
        let byte = bytes[len];
        if byte == 0 {
            saw_terminator = true;
            break;
        }
        if !is_valid_service_name_byte(byte) {
            return None;
        }
        len += 1;
    }

    if len == 0 {
        return None;
    }

    if saw_terminator && bytes[len + 1..].iter().any(|&byte| byte != 0) {
        return None;
    }

    core::str::from_utf8(&bytes[..len]).ok()
}

fn is_valid_service_name_byte(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-' | b'.')
}

const _: [(); 64] = [(); size_of::<NovaInitCapsuleHeaderV1>()];

#[cfg(test)]
mod tests {
    use super::{
        InitCapsuleImage, NOVA_INIT_CAPSULE_KNOWN_CAPABILITIES_V1,
        NOVA_INIT_CAPSULE_SERVICE_NAME_LEN, NovaInitCapsuleCapabilityV1, NovaInitCapsuleHeaderV1,
        decode_init_capsule_service_name, encode_init_capsule_service_name,
    };
    use crate::{NovaPayloadEntryAbi, NovaPayloadHeaderV1, NovaPayloadKind};
    use alloc::vec;
    use core::mem::{offset_of, size_of};

    #[test]
    fn init_capsule_layout_matches_c_header() {
        assert_eq!(size_of::<NovaInitCapsuleHeaderV1>(), 64);
        assert_eq!(offset_of!(NovaInitCapsuleHeaderV1, service_name), 40);
    }

    #[test]
    fn service_name_encoding_requires_valid_bootstrap_name() {
        assert!(encode_init_capsule_service_name("").is_none());
        assert!(encode_init_capsule_service_name("initd").is_some());
        assert!(encode_init_capsule_service_name("initd/worker").is_none());
        assert!(encode_init_capsule_service_name("abcdefghijklmnopq").is_none());
        assert_eq!(
            decode_init_capsule_service_name(
                &encode_init_capsule_service_name("initd").expect("name")
            ),
            Some("initd")
        );
    }

    #[test]
    fn init_capsule_image_parses_header_only_capsule() {
        let service_name = encode_init_capsule_service_name("initd").expect("service name");
        let header = NovaInitCapsuleHeaderV1::new(
            service_name,
            NovaInitCapsuleCapabilityV1::BootLog as u64,
            1,
            0,
        );
        let mut image = vec![0u8; size_of::<NovaInitCapsuleHeaderV1>()];
        image.copy_from_slice(unsafe {
            core::slice::from_raw_parts(
                &header as *const NovaInitCapsuleHeaderV1 as *const u8,
                size_of::<NovaInitCapsuleHeaderV1>(),
            )
        });

        let capsule = InitCapsuleImage::parse(image.as_slice()).expect("capsule");
        assert_eq!(capsule.service_name(), "initd");
        assert_eq!(
            capsule.requested_capabilities(),
            NovaInitCapsuleCapabilityV1::BootLog as u64
        );
        assert_eq!(capsule.endpoint_slots(), 1);
        assert_eq!(capsule.shared_memory_regions(), 0);
        assert!(capsule.body().is_empty());
        assert!(capsule.bootstrap_service_payload().is_none());
    }

    #[test]
    fn init_capsule_parses_embedded_bootstrap_service_payload() {
        let service_name = encode_init_capsule_service_name("initd").expect("service name");
        let payload_body = [0x41u8, 0x42, 0x43, 0x44];
        let payload_header = NovaPayloadHeaderV1::new_flat_binary(
            NovaPayloadKind::Service,
            NovaPayloadEntryAbi::BootstrapTaskV1,
            (size_of::<NovaPayloadHeaderV1>() + payload_body.len()) as u32,
            crate::sha256_digest_bytes(&payload_body),
        );
        let mut payload = vec![0u8; size_of::<NovaPayloadHeaderV1>() + payload_body.len()];
        payload[..size_of::<NovaPayloadHeaderV1>()].copy_from_slice(unsafe {
            core::slice::from_raw_parts(
                &payload_header as *const NovaPayloadHeaderV1 as *const u8,
                size_of::<NovaPayloadHeaderV1>(),
            )
        });
        payload[size_of::<NovaPayloadHeaderV1>()..].copy_from_slice(&payload_body);

        let mut header = NovaInitCapsuleHeaderV1::new(
            service_name,
            NovaInitCapsuleCapabilityV1::BootLog as u64,
            1,
            0,
        );
        header.total_size = (size_of::<NovaInitCapsuleHeaderV1>() + payload.len()) as u32;

        let mut image = vec![0u8; header.total_size as usize];
        image[..size_of::<NovaInitCapsuleHeaderV1>()].copy_from_slice(unsafe {
            core::slice::from_raw_parts(
                &header as *const NovaInitCapsuleHeaderV1 as *const u8,
                size_of::<NovaInitCapsuleHeaderV1>(),
            )
        });
        image[size_of::<NovaInitCapsuleHeaderV1>()..].copy_from_slice(&payload);

        let capsule = InitCapsuleImage::parse(image.as_slice()).expect("capsule");
        let embedded = capsule
            .bootstrap_service_payload()
            .expect("embedded payload");
        assert_eq!(embedded.kind(), NovaPayloadKind::Service);
        assert_eq!(embedded.entry_abi(), NovaPayloadEntryAbi::BootstrapTaskV1);
        assert_eq!(embedded.body(), payload_body);
    }

    #[test]
    fn init_capsule_rejects_unknown_capability_bits() {
        let service_name = encode_init_capsule_service_name("initd").expect("service name");
        let header = NovaInitCapsuleHeaderV1::new(
            service_name,
            NOVA_INIT_CAPSULE_KNOWN_CAPABILITIES_V1 | (1 << 12),
            0,
            0,
        );
        let image = unsafe {
            core::slice::from_raw_parts(
                &header as *const NovaInitCapsuleHeaderV1 as *const u8,
                size_of::<NovaInitCapsuleHeaderV1>(),
            )
        };

        assert!(InitCapsuleImage::parse(image).is_none());
    }

    #[test]
    fn init_capsule_rejects_invalid_embedded_payload_body() {
        let service_name = encode_init_capsule_service_name("initd").expect("service name");
        let mut header = NovaInitCapsuleHeaderV1::new(
            service_name,
            NovaInitCapsuleCapabilityV1::BootLog as u64,
            0,
            0,
        );
        let invalid_body = [1u8, 2, 3, 4];
        header.total_size = (size_of::<NovaInitCapsuleHeaderV1>() + invalid_body.len()) as u32;

        let mut image = vec![0u8; header.total_size as usize];
        image[..size_of::<NovaInitCapsuleHeaderV1>()].copy_from_slice(unsafe {
            core::slice::from_raw_parts(
                &header as *const NovaInitCapsuleHeaderV1 as *const u8,
                size_of::<NovaInitCapsuleHeaderV1>(),
            )
        });
        image[size_of::<NovaInitCapsuleHeaderV1>()..].copy_from_slice(&invalid_body);

        assert!(InitCapsuleImage::parse(image.as_slice()).is_none());
    }

    #[test]
    fn init_capsule_constants_cover_current_contract() {
        assert_eq!(NOVA_INIT_CAPSULE_SERVICE_NAME_LEN, 16);
        assert_eq!(NovaInitCapsuleCapabilityV1::BootLog as u64, 1);
        assert_eq!(NovaInitCapsuleCapabilityV1::Yield as u64, 2);
        assert_eq!(NovaInitCapsuleCapabilityV1::EndpointBootstrap as u64, 4);
        assert_eq!(NovaInitCapsuleCapabilityV1::SharedMemoryBootstrap as u64, 8);
    }
}
