#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${ROOT_DIR:-$(cd -- "${SCRIPT_DIR}/.." && pwd)}"
TARGET="${TARGET:-aarch64-unknown-none-softfloat}"
PROFILE="${PROFILE:-dev}"
INITD_FEATURES="${INITD_FEATURES:-${PAYLOAD_FEATURES:-}}"
export PATH="/home/linuxbrew/.linuxbrew/bin:/home/nova/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:${PATH:-}"

profile_dir="debug"
build_args=()
feature_args=()
if [ "${PROFILE}" = "release" ]; then
  build_args+=(--release)
  profile_dir="release"
fi

if [ -n "${INITD_FEATURES}" ]; then
  feature_args+=(--features "${INITD_FEATURES}")
fi

crate_dir="${ROOT_DIR}/apps/initd"
elf_path="${ROOT_DIR}/target/${TARGET}/${profile_dir}/initd-payload"
raw_bin_path="${ROOT_DIR}/target/${TARGET}/${profile_dir}/initd-payload.raw.bin"
bin_path="${ROOT_DIR}/target/${TARGET}/${profile_dir}/initd-payload.bin"

cargo rustc --manifest-path "${crate_dir}/Cargo.toml" --bin initd-payload --target "${TARGET}" \
  "${build_args[@]}" "${feature_args[@]}" \
  -- \
  -C linker=ld.lld \
  -C link-arg=-T"${ROOT_DIR}/apps/initd/link.ld"

llvm-objcopy -O binary "${elf_path}" "${raw_bin_path}"
cargo run -q -p novaos-mkimage -- \
  --kind service \
  --input "${raw_bin_path}" \
  --output "${bin_path}" >/dev/null

printf 'initd_payload_elf=%s\n' "${elf_path}"
printf 'initd_payload_raw_bin=%s\n' "${raw_bin_path}"
printf 'initd_payload_bin=%s\n' "${bin_path}"
