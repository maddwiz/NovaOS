use std::env;
use std::fs;
use std::mem::size_of;
use std::path::PathBuf;
use std::process::ExitCode;

use nova_rt::{
    InitCapsuleImage, NovaInitCapsuleHeaderV1, NovaPayloadEntryAbi, NovaPayloadHeaderV1,
    NovaPayloadKind, PayloadImage, encode_init_capsule_service_name, sha256_digest_bytes,
};

const KERNEL_PAYLOAD_LOAD_ALIGNMENT: usize = 2048;

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("novaos-mkimage: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    let options = Options::parse(args)?;

    match options.command {
        Command::Payload {
            check_only,
            kind,
            input,
            output,
        } => {
            if check_only {
                let image = fs::read(&input)
                    .map_err(|error| format!("failed to read {}: {error}", input.display()))?;
                let payload = PayloadImage::parse_kind(&image, kind)
                    .ok_or_else(|| format!("invalid {:?} payload image", kind))?;

                println!(
                    "kind={:?} image_size={} load_size={} load_offset={} entry_offset={} entry_abi={:?}",
                    payload.kind(),
                    payload.header().image_size,
                    payload.load_size(),
                    payload.header().load_offset,
                    payload.header().entry_offset,
                    payload.entry_abi(),
                );
                return Ok(());
            }

            let output =
                output.ok_or_else(|| String::from("missing --output for image creation"))?;
            let raw = fs::read(&input)
                .map_err(|error| format!("failed to read {}: {error}", input.display()))?;
            if raw.is_empty() {
                return Err(format!(
                    "refusing to wrap empty payload {}",
                    input.display()
                ));
            }

            let header_size = size_of::<NovaPayloadHeaderV1>();
            let load_offset = payload_load_offset(kind, header_size);
            let image_size = load_offset
                .checked_add(raw.len())
                .ok_or_else(|| String::from("payload image size overflow"))?;
            let image_size = u32::try_from(image_size)
                .map_err(|_| String::from("payload image exceeds v1 size limits"))?;
            let load_offset = u32::try_from(load_offset)
                .map_err(|_| String::from("payload load offset exceeds v1 size limits"))?;
            let header = NovaPayloadHeaderV1::new_flat_binary_with_offsets(
                kind,
                entry_abi_for_kind(kind),
                image_size,
                load_offset,
                load_offset,
                sha256_digest_bytes(&raw),
            );

            let mut image = Vec::with_capacity(image_size as usize);
            image.extend_from_slice(header_as_bytes(&header));
            image.resize(load_offset as usize, 0);
            image.extend_from_slice(&raw);

            fs::write(&output, image)
                .map_err(|error| format!("failed to write {}: {error}", output.display()))?;

            println!(
                "kind={:?} input={} output={} image_size={} body_size={}",
                kind,
                input.display(),
                output.display(),
                image_size,
                raw.len(),
            );
            Ok(())
        }
        Command::InitCapsule {
            check_only,
            input,
            output,
            service_name,
            capabilities,
            endpoint_slots,
            shared_memory_regions,
            body_input,
        } => {
            if check_only {
                let input =
                    input.ok_or_else(|| String::from("missing --input for init capsule check"))?;
                let image = fs::read(&input)
                    .map_err(|error| format!("failed to read {}: {error}", input.display()))?;
                let capsule = InitCapsuleImage::parse(&image)
                    .ok_or_else(|| format!("invalid init capsule {}", input.display()))?;
                let bootstrap_payload = capsule.bootstrap_service_payload();

                println!(
                    "service_name={} capabilities={:#x} endpoint_slots={} shared_memory_regions={} total_size={} bootstrap_payload_present={} bootstrap_payload_load_size={}",
                    capsule.service_name(),
                    capsule.requested_capabilities(),
                    capsule.endpoint_slots(),
                    capsule.shared_memory_regions(),
                    capsule.header().total_size,
                    bootstrap_payload.is_some(),
                    bootstrap_payload
                        .map(|payload| payload.load_size())
                        .unwrap_or(0),
                );
                return Ok(());
            }

            let output =
                output.ok_or_else(|| String::from("missing --output for init capsule creation"))?;
            let service_name = service_name
                .ok_or_else(|| String::from("missing --service-name for init capsule creation"))?;
            let encoded_service_name = encode_init_capsule_service_name(&service_name)
                .ok_or_else(|| format!("invalid init capsule service name `{service_name}`"))?;
            let body = match body_input {
                Some(body_input) => {
                    let body = fs::read(&body_input).map_err(|error| {
                        format!("failed to read {}: {error}", body_input.display())
                    })?;
                    if PayloadImage::parse_kind_abi(
                        &body,
                        NovaPayloadKind::Service,
                        NovaPayloadEntryAbi::BootstrapTaskV1,
                    )
                    .is_none()
                    {
                        return Err(format!(
                            "invalid bootstrap service payload {}",
                            body_input.display()
                        ));
                    }
                    body
                }
                None => Vec::new(),
            };
            let total_size = size_of::<NovaInitCapsuleHeaderV1>()
                .checked_add(body.len())
                .ok_or_else(|| String::from("init capsule size overflow"))?;
            let total_size = u32::try_from(total_size)
                .map_err(|_| String::from("init capsule exceeds v1 size limits"))?;
            let mut header = NovaInitCapsuleHeaderV1::new(
                encoded_service_name,
                capabilities,
                endpoint_slots,
                shared_memory_regions,
            );
            header.total_size = total_size;
            let mut image = Vec::with_capacity(total_size as usize);
            image.extend_from_slice(init_capsule_header_as_bytes(&header));
            image.extend_from_slice(&body);

            fs::write(&output, image)
                .map_err(|error| format!("failed to write {}: {error}", output.display()))?;

            println!(
                "service_name={} capabilities={:#x} endpoint_slots={} shared_memory_regions={} bootstrap_payload_present={} output={}",
                service_name,
                capabilities,
                endpoint_slots,
                shared_memory_regions,
                !body.is_empty(),
                output.display(),
            );
            Ok(())
        }
    }
}

fn header_as_bytes(header: &NovaPayloadHeaderV1) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            header as *const NovaPayloadHeaderV1 as *const u8,
            size_of::<NovaPayloadHeaderV1>(),
        )
    }
}

fn init_capsule_header_as_bytes(header: &NovaInitCapsuleHeaderV1) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            header as *const NovaInitCapsuleHeaderV1 as *const u8,
            size_of::<NovaInitCapsuleHeaderV1>(),
        )
    }
}

struct Options {
    command: Command,
}

enum Command {
    Payload {
        check_only: bool,
        kind: NovaPayloadKind,
        input: PathBuf,
        output: Option<PathBuf>,
    },
    InitCapsule {
        check_only: bool,
        input: Option<PathBuf>,
        output: Option<PathBuf>,
        service_name: Option<String>,
        capabilities: u64,
        endpoint_slots: u32,
        shared_memory_regions: u32,
        body_input: Option<PathBuf>,
    },
}

fn entry_abi_for_kind(kind: NovaPayloadKind) -> NovaPayloadEntryAbi {
    match kind {
        NovaPayloadKind::Stage1 => NovaPayloadEntryAbi::Stage1Plan,
        NovaPayloadKind::Kernel => NovaPayloadEntryAbi::BootInfoV2Sidecar,
        NovaPayloadKind::Service => NovaPayloadEntryAbi::BootstrapTaskV1,
    }
}

fn payload_load_offset(kind: NovaPayloadKind, header_size: usize) -> usize {
    match kind {
        NovaPayloadKind::Kernel => align_up(header_size, KERNEL_PAYLOAD_LOAD_ALIGNMENT),
        NovaPayloadKind::Stage1 | NovaPayloadKind::Service => header_size,
    }
}

fn align_up(value: usize, alignment: usize) -> usize {
    debug_assert!(alignment.is_power_of_two());
    (value + (alignment - 1)) & !(alignment - 1)
}

impl Options {
    fn parse(args: Vec<String>) -> Result<Self, String> {
        let mut check_only = false;
        let mut kind = None;
        let mut input = None;
        let mut output = None;
        let mut init_capsule_v1 = false;
        let mut service_name = None;
        let mut capabilities = 0u64;
        let mut endpoint_slots = 0u32;
        let mut shared_memory_regions = 0u32;
        let mut body_input = None;

        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--check" => check_only = true,
                "--init-capsule-v1" => init_capsule_v1 = true,
                "--kind" => {
                    let value = iter
                        .next()
                        .ok_or_else(|| String::from("missing value for --kind"))?;
                    kind = Some(parse_kind(&value)?);
                }
                "--input" => {
                    let value = iter
                        .next()
                        .ok_or_else(|| String::from("missing value for --input"))?;
                    input = Some(PathBuf::from(value));
                }
                "--output" => {
                    let value = iter
                        .next()
                        .ok_or_else(|| String::from("missing value for --output"))?;
                    output = Some(PathBuf::from(value));
                }
                "--service-name" => {
                    let value = iter
                        .next()
                        .ok_or_else(|| String::from("missing value for --service-name"))?;
                    service_name = Some(value);
                }
                "--capabilities" => {
                    let value = iter
                        .next()
                        .ok_or_else(|| String::from("missing value for --capabilities"))?;
                    capabilities = parse_u64(&value, "--capabilities")?;
                }
                "--endpoint-slots" => {
                    let value = iter
                        .next()
                        .ok_or_else(|| String::from("missing value for --endpoint-slots"))?;
                    endpoint_slots = parse_u32(&value, "--endpoint-slots")?;
                }
                "--shared-memory-regions" => {
                    let value = iter
                        .next()
                        .ok_or_else(|| String::from("missing value for --shared-memory-regions"))?;
                    shared_memory_regions = parse_u32(&value, "--shared-memory-regions")?;
                }
                "--body-input" => {
                    let value = iter
                        .next()
                        .ok_or_else(|| String::from("missing value for --body-input"))?;
                    body_input = Some(PathBuf::from(value));
                }
                "--help" | "-h" => return Err(usage()),
                other => return Err(format!("unexpected argument `{other}`")),
            }
        }

        let command = if init_capsule_v1 {
            Command::InitCapsule {
                check_only,
                input,
                output,
                service_name,
                capabilities,
                endpoint_slots,
                shared_memory_regions,
                body_input,
            }
        } else {
            Command::Payload {
                check_only,
                kind: kind.ok_or_else(usage)?,
                input: input.ok_or_else(usage)?,
                output,
            }
        };

        Ok(Self { command })
    }
}

fn parse_kind(value: &str) -> Result<NovaPayloadKind, String> {
    match value {
        "stage1" => Ok(NovaPayloadKind::Stage1),
        "kernel" => Ok(NovaPayloadKind::Kernel),
        "service" => Ok(NovaPayloadKind::Service),
        _ => Err(format!("unsupported payload kind `{value}`")),
    }
}

fn usage() -> String {
    String::from(
        "usage: novaos-mkimage [--check] --kind <stage1|kernel|service> --input <path> [--output <path>]\n       novaos-mkimage [--check] --init-capsule-v1 [--input <path>] [--output <path>] [--service-name <name>] [--capabilities <value>] [--endpoint-slots <count>] [--shared-memory-regions <count>] [--body-input <path>]",
    )
}

fn parse_u64(value: &str, flag: &str) -> Result<u64, String> {
    if let Some(value) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        u64::from_str_radix(value, 16).map_err(|_| format!("invalid value for {flag}"))
    } else {
        value
            .parse::<u64>()
            .map_err(|_| format!("invalid value for {flag}"))
    }
}

fn parse_u32(value: &str, flag: &str) -> Result<u32, String> {
    if let Some(value) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        u32::from_str_radix(value, 16).map_err(|_| format!("invalid value for {flag}"))
    } else {
        value
            .parse::<u32>()
            .map_err(|_| format!("invalid value for {flag}"))
    }
}
