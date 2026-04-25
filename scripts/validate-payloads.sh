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
PROFILE="${PROFILE}" bash "${ROOT_DIR}/scripts/build-policyd.sh" >/dev/null

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
