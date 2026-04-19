#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${ROOT_DIR:-/home/nova/NovaOS}"
export PATH="/home/linuxbrew/.linuxbrew/bin:/home/nova/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:${PATH:-}"

printf 'ROOT_DIR=%s\n' "${ROOT_DIR}"
printf 'RUST_TARGET=aarch64-unknown-uefi\n'
printf 'QEMU_BINARY=%s\n' "$(command -v qemu-system-aarch64 2>/dev/null || true)"
printf 'QEMU_FIRMWARE=%s\n' "$(
  for candidate in \
    /home/linuxbrew/.linuxbrew/share/qemu/edk2-aarch64-code.fd \
    /home/linuxbrew/.linuxbrew/Cellar/qemu/10.2.2/share/qemu/edk2-aarch64-code.fd \
    /usr/share/qemu-efi-aarch64/QEMU_EFI.fd
  do
    if [ -f "${candidate}" ]; then
      printf '%s' "${candidate}"
      break
    fi
  done
)"
printf 'QEMU_VARS=%s\n' "$(
  for candidate in \
    /home/linuxbrew/.linuxbrew/share/qemu/edk2-arm-vars.fd \
    /home/linuxbrew/.linuxbrew/Cellar/qemu/10.2.2/share/qemu/edk2-arm-vars.fd
  do
    if [ -f "${candidate}" ]; then
      printf '%s' "${candidate}"
      break
    fi
  done
)"
printf 'LLVM_OBJCOPY=%s\n' "$(command -v llvm-objcopy 2>/dev/null || true)"
