#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${ROOT_DIR:-$(cd -- "${SCRIPT_DIR}/.." && pwd)}"
CRATE_DIR="${CRATE_DIR:-}"
BIN_NAME="${BIN_NAME:-}"
OUTPUT_STEM="${OUTPUT_STEM:-${BIN_NAME}}"
TARGET="${TARGET:-aarch64-unknown-none-softfloat}"
PROFILE="${PROFILE:-dev}"
LINKER_SCRIPT="${LINKER_SCRIPT:-${ROOT_DIR}/apps/initd/link.ld}"
SERVICE_PAYLOAD_FEATURES="${SERVICE_PAYLOAD_FEATURES:-${PAYLOAD_FEATURES:-}}"
export PATH="/home/linuxbrew/.linuxbrew/bin:/home/nova/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:${PATH:-}"

if [ -z "${CRATE_DIR}" ]; then
  printf 'build-service-payload: missing CRATE_DIR\n' >&2
  exit 1
fi

if [ -z "${BIN_NAME}" ]; then
  printf 'build-service-payload: missing BIN_NAME\n' >&2
  exit 1
fi

profile_dir="debug"
build_args=()
feature_args=()
if [ "${PROFILE}" = "release" ]; then
  build_args+=(--release)
  profile_dir="release"
fi

if [ -n "${SERVICE_PAYLOAD_FEATURES}" ]; then
  feature_args+=(--features "${SERVICE_PAYLOAD_FEATURES}")
fi

elf_path="${ROOT_DIR}/target/${TARGET}/${profile_dir}/${OUTPUT_STEM}"
raw_bin_path="${ROOT_DIR}/target/${TARGET}/${profile_dir}/${OUTPUT_STEM}.raw.bin"
bin_path="${ROOT_DIR}/target/${TARGET}/${profile_dir}/${OUTPUT_STEM}.bin"

cargo rustc --manifest-path "${CRATE_DIR}/Cargo.toml" --bin "${BIN_NAME}" --target "${TARGET}" \
  "${build_args[@]}" "${feature_args[@]}" \
  -- \
  -C linker=ld.lld \
  -C link-arg=-T"${LINKER_SCRIPT}"

llvm-objcopy -O binary "${elf_path}" "${raw_bin_path}"
cargo run -q -p novaos-mkimage -- \
  --kind service \
  --input "${raw_bin_path}" \
  --output "${bin_path}" >/dev/null

printf 'payload_elf=%s\n' "${elf_path}"
printf 'payload_raw_bin=%s\n' "${raw_bin_path}"
printf 'payload_bin=%s\n' "${bin_path}"
