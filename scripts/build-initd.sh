#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${ROOT_DIR:-$(cd -- "${SCRIPT_DIR}/.." && pwd)}"
TARGET="${TARGET:-aarch64-unknown-none-softfloat}"
PROFILE="${PROFILE:-dev}"
INITD_FEATURES="${INITD_FEATURES:-${PAYLOAD_FEATURES:-}}"
export PATH="/home/linuxbrew/.linuxbrew/bin:/home/nova/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:${PATH:-}"
output="$(
  CRATE_DIR="${ROOT_DIR}/apps/initd" \
  BIN_NAME="initd-payload" \
  OUTPUT_STEM="initd-payload" \
  TARGET="${TARGET}" \
  PROFILE="${PROFILE}" \
  LINKER_SCRIPT="${ROOT_DIR}/apps/initd/link.ld" \
  SERVICE_PAYLOAD_FEATURES="${INITD_FEATURES}" \
  bash "${ROOT_DIR}/scripts/build-service-payload.sh"
)"

printf '%s\n' "${output}" \
  | sed \
      -e 's/^payload_elf=/initd_payload_elf=/' \
      -e 's/^payload_raw_bin=/initd_payload_raw_bin=/' \
      -e 's/^payload_bin=/initd_payload_bin=/'
