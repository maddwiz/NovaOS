#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${ROOT_DIR:-/home/nova/NovaOS}"
TARGET="${TARGET:-aarch64-unknown-uefi}"
PROFILE="${PROFILE:-dev}"
NOVAAA64_FEATURES="${NOVAAA64_FEATURES:-}"
export PATH="/home/linuxbrew/.linuxbrew/bin:/home/nova/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:${PATH:-}"

build_args=()
novaaa64_feature_args=()
profile_dir="debug"
if [ "${PROFILE}" = "release" ]; then
  build_args+=(--release)
  profile_dir="release"
fi

if [ -n "${NOVAAA64_FEATURES}" ]; then
  novaaa64_feature_args+=(--features "${NOVAAA64_FEATURES}")
fi

cargo build -p spark-observe --target "${TARGET}" "${build_args[@]}"
cargo build -p novaaa64 --target "${TARGET}" "${build_args[@]}" "${novaaa64_feature_args[@]}"

printf 'spark_observe_efi=%s\n' "${ROOT_DIR}/target/${TARGET}/${profile_dir}/spark-observe.efi"
printf 'novaaa64_efi=%s\n' "${ROOT_DIR}/target/${TARGET}/${profile_dir}/novaaa64.efi"
