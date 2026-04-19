#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${ROOT_DIR:-/home/nova/NovaOS}"
TARGET="${TARGET:-aarch64-unknown-none-softfloat}"
PROFILE="${PROFILE:-dev}"
PAYLOAD_FEATURES="${PAYLOAD_FEATURES:-}"
export PATH="/home/linuxbrew/.linuxbrew/bin:/home/nova/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:${PATH:-}"

profile_dir="debug"
build_args=()
feature_args=()
if [ "${PROFILE}" = "release" ]; then
  build_args+=(--release)
  profile_dir="release"
fi

if [ -n "${PAYLOAD_FEATURES}" ]; then
  feature_args+=(--features "${PAYLOAD_FEATURES}")
fi

build_payload() {
  local package_name="$1"
  local binary_name="$2"
  local linker_script="$3"
  local payload_kind="$4"
  local elf_path="${ROOT_DIR}/target/${TARGET}/${profile_dir}/${binary_name}"
  local raw_bin_path="${ROOT_DIR}/target/${TARGET}/${profile_dir}/${binary_name}.raw.bin"
  local bin_path="${ROOT_DIR}/target/${TARGET}/${profile_dir}/${binary_name}.bin"

  cargo rustc -p "${package_name}" --bin "${binary_name}" --target "${TARGET}" \
    "${build_args[@]}" "${feature_args[@]}" -- \
    -C linker=ld.lld \
    -C link-arg=-T"${linker_script}"

  llvm-objcopy -O binary "${elf_path}" "${raw_bin_path}"
  cargo run -q -p novaos-mkimage -- \
    --kind "${payload_kind}" \
    --input "${raw_bin_path}" \
    --output "${bin_path}" >/dev/null
}

build_payload \
  "novaos-stage1" \
  "stage1-payload" \
  "${ROOT_DIR}/boot/stage1/link.ld" \
  "stage1"

build_payload \
  "novaos-kernel" \
  "kernel-payload" \
  "${ROOT_DIR}/kernel/arch/arm64/link.ld" \
  "kernel"

printf 'novaos_stage1_elf=%s\n' "${ROOT_DIR}/target/${TARGET}/${profile_dir}/stage1-payload"
printf 'novaos_stage1_bin=%s\n' "${ROOT_DIR}/target/${TARGET}/${profile_dir}/stage1-payload.bin"
printf 'novaos_kernel_elf=%s\n' "${ROOT_DIR}/target/${TARGET}/${profile_dir}/kernel-payload"
printf 'novaos_kernel_bin=%s\n' "${ROOT_DIR}/target/${TARGET}/${profile_dir}/kernel-payload.bin"
