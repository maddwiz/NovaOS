#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${ROOT_DIR:-$(cd -- "${SCRIPT_DIR}/.." && pwd)}"
TARGET="${TARGET:-aarch64-unknown-none-softfloat}"
PROFILE="${PROFILE:-dev}"
export PATH="/home/linuxbrew/.linuxbrew/bin:/home/nova/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:${PATH:-}"
output="$(
  CRATE_DIR="${ROOT_DIR}/services/policyd" \
  BIN_NAME="policyd-payload" \
  OUTPUT_STEM="policyd-payload" \
  TARGET="${TARGET}" \
  PROFILE="${PROFILE}" \
  LINKER_SCRIPT="${ROOT_DIR}/services/policyd/link.ld" \
  bash "${ROOT_DIR}/scripts/build-service-payload.sh"
)"

printf '%s\n' "${output}" \
  | sed \
      -e 's/^payload_elf=/policyd_payload_elf=/' \
      -e 's/^payload_raw_bin=/policyd_payload_raw_bin=/' \
      -e 's/^payload_bin=/policyd_payload_bin=/'
