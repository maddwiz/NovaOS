#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${ROOT_DIR:-/home/nova/NovaOS}"
PROFILE="${PROFILE:-dev}"
OUTPUT_PATH="${OUTPUT_PATH:-${1:-}}"
SERVICE_NAME="${SERVICE_NAME:-initd}"
CAPABILITIES="${CAPABILITIES:-0xd}"
ENDPOINT_SLOTS="${ENDPOINT_SLOTS:-1}"
SHARED_MEMORY_REGIONS="${SHARED_MEMORY_REGIONS:-1}"
EMBED_BOOTSTRAP_PAYLOAD="${EMBED_BOOTSTRAP_PAYLOAD:-1}"
BOOTSTRAP_PAYLOAD_FILE="${BOOTSTRAP_PAYLOAD_FILE:-}"
INITD_FEATURES="${INITD_FEATURES:-}"
export PATH="/home/linuxbrew/.linuxbrew/bin:/home/nova/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:${PATH:-}"

profile_dir="debug"
run_args=(-q -p novaos-mkimage)
if [ "${PROFILE}" = "release" ]; then
  profile_dir="release"
  run_args+=(--release)
fi

if [ -z "${OUTPUT_PATH}" ]; then
  OUTPUT_PATH="${ROOT_DIR}/target/init-capsule/${profile_dir}/init.capsule"
fi

mkdir -p "$(dirname "${OUTPUT_PATH}")"

body_input_args=()
if [ "${EMBED_BOOTSTRAP_PAYLOAD}" = "1" ]; then
  if [ -z "${BOOTSTRAP_PAYLOAD_FILE}" ]; then
    PROFILE="${PROFILE}" INITD_FEATURES="${INITD_FEATURES}" \
      bash "${ROOT_DIR}/scripts/build-initd.sh" >/dev/null
    BOOTSTRAP_PAYLOAD_FILE="${ROOT_DIR}/target/aarch64-unknown-none-softfloat/${profile_dir}/initd-payload.bin"
  fi
  body_input_args+=(--body-input "${BOOTSTRAP_PAYLOAD_FILE}")
fi

cargo run "${run_args[@]}" -- \
  --init-capsule-v1 \
  --service-name "${SERVICE_NAME}" \
  --capabilities "${CAPABILITIES}" \
  --endpoint-slots "${ENDPOINT_SLOTS}" \
  --shared-memory-regions "${SHARED_MEMORY_REGIONS}" \
  "${body_input_args[@]}" \
  --output "${OUTPUT_PATH}" \
  >/dev/null

cargo run "${run_args[@]}" -- \
  --check \
  --init-capsule-v1 \
  --input "${OUTPUT_PATH}" \
  >/dev/null

printf 'init_capsule=%s\n' "${OUTPUT_PATH}"
printf 'service_name=%s\n' "${SERVICE_NAME}"
printf 'capabilities=%s\n' "${CAPABILITIES}"
printf 'endpoint_slots=%s\n' "${ENDPOINT_SLOTS}"
printf 'shared_memory_regions=%s\n' "${SHARED_MEMORY_REGIONS}"
printf 'bootstrap_payload=%s\n' "${BOOTSTRAP_PAYLOAD_FILE:-none}"
