#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${ROOT_DIR:-/home/nova/NovaOS}"
ARTIFACT_DIR="${ARTIFACT_DIR:-${ROOT_DIR}/artifacts/hardware}"
MODE="${MODE:-${1:-}}"
BUNDLE_KIND="${BUNDLE_KIND:-}"
PROFILE="${PROFILE:-dev}"
OUTPUT_DIR="${OUTPUT_DIR:-}"
INIT_CAPSULE_FILE="${INIT_CAPSULE_FILE:-}"
PAYLOAD_FEATURES="${PAYLOAD_FEATURES:-}"
NOVAAA64_FEATURES="${NOVAAA64_FEATURES:-}"
export PATH="/home/linuxbrew/.linuxbrew/bin:/home/nova/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:${PATH:-}"
source "${ROOT_DIR}/scripts/novaos-latest.sh"

profile_dir="debug"
if [ "${PROFILE}" = "release" ]; then
  profile_dir="release"
fi

if [ -n "${MODE}" ]; then
  case "${MODE}" in
    spark-observe)
      BUNDLE_KIND="observatory"
      ;;
    novaaa64)
      BUNDLE_KIND="loader"
      ;;
    observatory|loader)
      BUNDLE_KIND="${MODE}"
      ;;
    *)
      printf 'unsupported_mode=%s\n' "${MODE}" >&2
      exit 1
      ;;
  esac
fi

if [ -z "${BUNDLE_KIND}" ]; then
  BUNDLE_KIND="observatory"
fi

case "${BUNDLE_KIND}" in
  observatory)
    bundle_prefix="spark-observe"
    efi_binary="spark-observe"
    mode_name="spark-observe"
    include_payloads=0
    ;;
  loader)
    bundle_prefix="novaaa64"
    efi_binary="novaaa64"
    mode_name="novaaa64"
    include_payloads=1
    ;;
  *)
    printf 'unsupported_bundle_kind=%s\n' "${BUNDLE_KIND}" >&2
    exit 1
    ;;
esac

bash "${ROOT_DIR}/scripts/build-efi.sh" >/dev/null
if [ "${include_payloads}" = "1" ]; then
  PROFILE="${PROFILE}" PAYLOAD_FEATURES="${PAYLOAD_FEATURES}" \
    bash "${ROOT_DIR}/scripts/build-kernel.sh" >/dev/null
fi

stamp="$(date -u +%Y%m%dT%H%M%SZ)"
bundle_dir="${OUTPUT_DIR:-${ARTIFACT_DIR}/${bundle_prefix}-${stamp}}"
instructions_file="${bundle_dir}/README.txt"
manifest_file="${bundle_dir}/bundle-manifest.txt"
status_file="${ARTIFACT_DIR}/latest-${bundle_prefix}-bundle-status.txt"
path_file="${ARTIFACT_DIR}/latest-${bundle_prefix}-bundle-path.txt"
latest_link="${ARTIFACT_DIR}/latest-${bundle_prefix}-bundle"

if [ -e "${bundle_dir}" ]; then
  printf 'output_exists=%s\n' "${bundle_dir}" >&2
  exit 1
fi

mkdir -p "${bundle_dir}/EFI/BOOT"
cp "${ROOT_DIR}/target/aarch64-unknown-uefi/${profile_dir}/${efi_binary}.efi" \
  "${bundle_dir}/EFI/BOOT/BOOTAA64.EFI"
cp "${ROOT_DIR}/target/aarch64-unknown-uefi/${profile_dir}/${efi_binary}.efi" \
  "${bundle_dir}/${efi_binary}.efi"

stage1_bin=""
kernel_bin=""
init_capsule=""
if [ "${include_payloads}" = "1" ]; then
  mkdir -p "${bundle_dir}/nova"
  stage1_bin="${bundle_dir}/nova/stage1.bin"
  kernel_bin="${bundle_dir}/nova/kernel.bin"
  init_capsule="${bundle_dir}/nova/init.capsule"
  cp "${ROOT_DIR}/target/aarch64-unknown-none-softfloat/${profile_dir}/stage1-payload.bin" \
    "${stage1_bin}"
  cp "${ROOT_DIR}/target/aarch64-unknown-none-softfloat/${profile_dir}/kernel-payload.bin" \
    "${kernel_bin}"
  if [ -n "${INIT_CAPSULE_FILE}" ]; then
    cp "${INIT_CAPSULE_FILE}" "${init_capsule}"
  else
    PROFILE="${PROFILE}" bash "${ROOT_DIR}/scripts/build-init-capsule.sh" "${init_capsule}" >/dev/null
  fi
fi

cat > "${instructions_file}" <<EOF
NovaOS Spark hardware bundle
prepared_at_utc=${stamp}
mode=${mode_name}
bundle_kind=${BUNDLE_KIND}
profile=${PROFILE}

This bundle is staged for manual Spark hardware validation.
It does not write to /boot/efi or reboot the machine.

Suggested operator flow
1. Preferred Linux operator path: run sudo -E bash ./scripts/install-spark-hardware-bundle.sh ${mode_name} to copy this bundle onto the mounted ESP.
EOF

if [ "${include_payloads}" = "1" ]; then
  cat >> "${instructions_file}" <<EOF
2. Optional one-time boot path: rerun the install step with USE_BOOTNEXT=1, and optionally REBOOT_AFTER=1, if the operator wants the script to schedule the next UEFI boot.
3. Manual fallback: copy the contents of this directory to a FAT USB stick or ESP so \EFI\BOOT\BOOTAA64.EFI exists.
4. Keep \nova\stage1.bin, \nova\kernel.bin, and \nova\init.capsule together with the EFI binary.
5. Use Spark UEFI one-time boot to launch the prepared media.
6. After boot, collect \nova\loader\novaaa64-loader-report.txt or \EFI\BOOT\novaaa64-loader-report.txt and ingest it with bash ./scripts/collect-novaaa64-loader-report.sh.
7. If the run also exposes stage0 -> stage1 -> kernel proof through serial, display capture, or another log path, bring that evidence back too.
8. Once the observatory return and stage-chain evidence are also back in the repo, prefer bash ./scripts/complete-spark-hardware-proof.sh /path/to/stage-chain-evidence.txt to ingest both returns, validate the stage-chain markers, and publish the combined proof.

Prepared files
- EFI/BOOT/BOOTAA64.EFI
EOF
else
  cat >> "${instructions_file}" <<EOF
2. Optional one-time boot path: rerun the install step with USE_BOOTNEXT=1, and optionally REBOOT_AFTER=1, if the operator wants the script to schedule the next UEFI boot.
3. Manual fallback: copy the contents of this directory to a FAT USB stick or ESP so \EFI\BOOT\BOOTAA64.EFI exists.
4. Use Spark UEFI one-time boot to launch the prepared media.
5. After boot, collect \nova\observatory\spark-observe-report.txt or \EFI\BOOT\spark-observe-report.txt and ingest it with bash ./scripts/collect-spark-observe-report.sh.
6. Once the loader return and stage-chain evidence are also back in the repo, prefer bash ./scripts/complete-spark-hardware-proof.sh /path/to/stage-chain-evidence.txt to ingest both returns, validate the stage-chain markers, and publish the combined proof.

Prepared files
- EFI/BOOT/BOOTAA64.EFI
EOF
fi

if [ "${include_payloads}" = "1" ]; then
  cat >> "${instructions_file}" <<EOF
- nova/stage1.bin
- nova/kernel.bin
- nova/init.capsule
EOF
fi

cat > "${manifest_file}" <<EOF
generated_at_utc=${stamp}
mode=${mode_name}
bundle_kind=${BUNDLE_KIND}
profile=${PROFILE}
bundle_dir=${bundle_dir}
bootaa64_efi=${bundle_dir}/EFI/BOOT/BOOTAA64.EFI
efi_binary=${efi_binary}
has_nova_payloads=${include_payloads}
stage1_bin=${stage1_bin}
kernel_bin=${kernel_bin}
init_capsule=${init_capsule}
instructions_file=${instructions_file}
EOF

novaos_refresh_latest_link "${bundle_dir}" "${latest_link}"
novaos_write_latest_path "${bundle_dir}" "${path_file}"
novaos_write_latest_status "${status_file}" \
  "generated_at_utc=${stamp}" \
  "mode=${mode_name}" \
  "bundle_kind=${BUNDLE_KIND}" \
  "bundle_dir=${bundle_dir}" \
  "efi_binary=${efi_binary}" \
  "profile=${PROFILE}" \
  "latest_bundle_link=${latest_link}" \
  "manifest_file=${manifest_file}" \
  "instructions_file=${instructions_file}"

printf 'mode=%s\n' "${mode_name}"
printf 'bundle_kind=%s\n' "${BUNDLE_KIND}"
printf 'bundle_dir=%s\n' "${bundle_dir}"
printf 'efi_binary=%s\n' "${efi_binary}"
printf 'bootaa64_efi=%s\n' "${bundle_dir}/EFI/BOOT/BOOTAA64.EFI"
if [ "${include_payloads}" = "1" ]; then
  printf 'stage1_bin=%s\n' "${stage1_bin}"
  printf 'kernel_bin=%s\n' "${kernel_bin}"
  printf 'init_capsule=%s\n' "${init_capsule}"
fi
printf 'manifest_file=%s\n' "${manifest_file}"
printf 'instructions_file=%s\n' "${instructions_file}"
