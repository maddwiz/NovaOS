#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${ROOT_DIR:-$(cd -- "${SCRIPT_DIR}/.." && pwd)}"
TARGET="${TARGET:-aarch64-unknown-none-softfloat}"
PROFILE="${PROFILE:-dev}"
export PATH="/home/linuxbrew/.linuxbrew/bin:/home/nova/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:${PATH:-}"

profile_dir="debug"
if [ "${PROFILE}" = "release" ]; then
  profile_dir="release"
fi

PROFILE="${PROFILE}" bash "${ROOT_DIR}/scripts/build-initd.sh" >/dev/null

build_runtime_service_payload() {
  local crate_dir="$1"
  local bin_name="$2"
  local stem="$3"

  CRATE_DIR="${ROOT_DIR}/${crate_dir}" \
    BIN_NAME="${bin_name}" \
    OUTPUT_STEM="${stem}" \
    PROFILE="${PROFILE}" \
    bash "${ROOT_DIR}/scripts/build-service-payload.sh" >/dev/null
}

build_runtime_service_payload "services/policyd" "policyd-payload" "policyd-payload"
build_runtime_service_payload "services/agentd" "agentd-payload" "agentd-payload"
build_runtime_service_payload "services/memd" "memd-payload" "memd-payload"
build_runtime_service_payload "services/acceld" "acceld-payload" "acceld-payload"
build_runtime_service_payload "services/intentd" "intentd-payload" "intentd-payload"
build_runtime_service_payload "services/scened" "scened-payload" "scened-payload"
build_runtime_service_payload "services/appbridged" "appbridged-payload" "appbridged-payload"
build_runtime_service_payload "services/shelld" "shelld-payload" "shelld-payload"

check_payload() {
  local stem="$1"
  local kind="$2"
  local elf_path="${ROOT_DIR}/target/${TARGET}/${profile_dir}/${stem}"
  local raw_bin_path="${elf_path}.raw.bin"
  local bin_path="${elf_path}.bin"
  local header
  local reloc
  local symbol
  local bin_size

  header="$(readelf -h "${elf_path}")"
  reloc="$(readelf -r "${elf_path}")"
  symbol="$(llvm-nm -n "${elf_path}" | sed -n '1p')"
  bin_size="$(wc -c < "${bin_path}")"

  printf '%s\n' "${header}" | grep -q "Type:.*EXEC"
  printf '%s\n' "${header}" | grep -q "Entry point address:.*0x0"
  printf '%s\n' "${symbol}" | grep -Eq '^0+ T _start$'

  if printf '%s\n' "${reloc}" | grep -q "Relocation section"; then
    printf 'payload %s has relocations\n' "${stem}" >&2
    exit 1
  fi

  if [ "${bin_size}" -le 0 ]; then
    printf 'payload %s has empty binary image\n' "${stem}" >&2
    exit 1
  fi

  cargo run -q -p novaos-mkimage -- --check --kind "${kind}" --input "${bin_path}" >/dev/null

  printf '%s_entry=pass\n' "${stem}"
  printf '%s_relocations=pass\n' "${stem}"
  printf '%s_header=pass\n' "${stem}"
  printf '%s_bin_size=%s\n' "${stem}" "${bin_size}"
}

check_payload "stage1-payload" "stage1"
check_payload "kernel-payload" "kernel"
check_payload "initd-payload" "service"
check_payload "policyd-payload" "service"
check_payload "agentd-payload" "service"
check_payload "memd-payload" "service"
check_payload "acceld-payload" "service"
check_payload "intentd-payload" "service"
check_payload "scened-payload" "service"
check_payload "appbridged-payload" "service"
check_payload "shelld-payload" "service"
