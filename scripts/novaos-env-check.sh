#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${ROOT_DIR:-$(cd -- "${SCRIPT_DIR}/.." && pwd)}"
export PATH="/home/linuxbrew/.linuxbrew/bin:/home/nova/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:${PATH:-}"

have() {
  command -v "$1" >/dev/null 2>&1
}

has_rust_target() {
  local target="$1"
  rustup_home="${RUSTUP_HOME:-${HOME}/.rustup}"
  toolchain_dir="${rustup_home}/toolchains"
  [ -d "${toolchain_dir}" ] || return 1
  find "${toolchain_dir}" -path "*/lib/rustlib/${target}" -type d 2>/dev/null | grep -q .
}

report_line() {
  printf '%s\n' "$1"
}

status=0

report_line "NovaOS environment check"
report_line "root=${ROOT_DIR}"

if have cargo; then
  report_line "cargo=present $(command -v cargo)"
else
  report_line "cargo=missing"
  status=1
fi

if have rustup; then
  for target in \
    aarch64-unknown-none-softfloat \
    aarch64-unknown-uefi \
    x86_64-unknown-none
  do
    if has_rust_target "${target}"; then
      report_line "rust_target=${target} present"
    else
      report_line "rust_target=${target} missing"
      status=1
    fi
  done
else
  report_line "rustup=missing"
  status=1
fi

if have qemu-system-aarch64; then
  report_line "qemu-system-aarch64=present"
else
  report_line "qemu-system-aarch64=missing"
  status=1
fi

firmware_found=0
for candidate in \
  /usr/share/AAVMF/AAVMF_CODE.fd \
  /usr/share/qemu-efi-aarch64/QEMU_EFI.fd \
  /usr/share/edk2/aarch64/QEMU_EFI.fd \
  /home/linuxbrew/.linuxbrew/Cellar/qemu/10.2.2/share/qemu/edk2-aarch64-code.fd \
  /home/linuxbrew/.linuxbrew/share/qemu/edk2-aarch64-code.fd \
  /usr/share/OVMF/OVMF_CODE.fd
do
  if [ -f "$candidate" ]; then
    report_line "uefi_firmware=${candidate}"
    firmware_found=1
    break
  fi
done

if [ "$firmware_found" -eq 0 ]; then
  report_line "uefi_firmware=missing"
  status=1
fi

vars_found=0
for candidate in \
  /usr/share/AAVMF/AAVMF_VARS.fd \
  /usr/share/qemu-efi-aarch64/QEMU_VARS.fd \
  /usr/share/qemu-efi-aarch64/QEMU_EFI_VARS.fd \
  /usr/share/edk2/aarch64/QEMU_VARS.fd \
  /home/linuxbrew/.linuxbrew/Cellar/qemu/10.2.2/share/qemu/edk2-arm-vars.fd \
  /home/linuxbrew/.linuxbrew/share/qemu/edk2-arm-vars.fd \
  /usr/share/OVMF/OVMF_VARS.fd
do
  if [ -f "$candidate" ]; then
    report_line "uefi_vars=${candidate}"
    vars_found=1
    break
  fi
done

if [ "$vars_found" -eq 0 ]; then
  report_line "uefi_vars=missing"
  status=1
fi

if have llvm-objcopy; then
  report_line "llvm-objcopy=present"
else
  report_line "llvm-objcopy=missing"
  status=1
fi

if have sbsign; then
  report_line "sbsign=present"
else
  report_line "sbsign=missing"
fi

exit "$status"
