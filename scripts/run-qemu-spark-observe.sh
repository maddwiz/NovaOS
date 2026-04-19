#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${ROOT_DIR:-/home/nova/NovaOS}"
PROFILE="${PROFILE:-dev}"
FIRMWARE_PATH="${FIRMWARE_PATH:-}"
FIRMWARE_VARS_PATH="${FIRMWARE_VARS_PATH:-}"
TIMEOUT_SECONDS="${TIMEOUT_SECONDS:-20}"
EFI_BINARY="${EFI_BINARY:-spark-observe}"
EXPECT_TEXT="${EXPECT_TEXT:-NovaOS Spark observe}"
EXPECT_TEXTS="${EXPECT_TEXTS:-}"
STAGE_NOVA_PAYLOADS="${STAGE_NOVA_PAYLOADS:-0}"
QEMU_TRACE="${QEMU_TRACE:-0}"
export PATH="/home/linuxbrew/.linuxbrew/bin:/home/nova/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:${PATH:-}"
source "${ROOT_DIR}/scripts/novaos-latest.sh"
profile_dir="debug"
payload_features="${PAYLOAD_FEATURES:-}"
initd_features="${INITD_FEATURES:-${payload_features}}"
qemu_args=()

if [ "${PROFILE}" = "release" ]; then
  profile_dir="release"
fi

if [ -z "${FIRMWARE_PATH}" ]; then
  for candidate in \
    /home/linuxbrew/.linuxbrew/share/qemu/edk2-aarch64-code.fd \
    /home/linuxbrew/.linuxbrew/Cellar/qemu/10.2.2/share/qemu/edk2-aarch64-code.fd \
    /usr/share/qemu-efi-aarch64/QEMU_EFI.fd
  do
    if [ -f "${candidate}" ]; then
      FIRMWARE_PATH="${candidate}"
      break
    fi
  done
fi

if [ -z "${FIRMWARE_PATH}" ] || [ ! -f "${FIRMWARE_PATH}" ]; then
  printf 'missing_qemu_firmware\n' >&2
  exit 1
fi

if [ -z "${FIRMWARE_VARS_PATH}" ]; then
  for candidate in \
    /home/linuxbrew/.linuxbrew/Cellar/qemu/10.2.2/share/qemu/edk2-arm-vars.fd \
    /home/linuxbrew/.linuxbrew/share/qemu/edk2-arm-vars.fd
  do
    if [ -f "${candidate}" ]; then
      FIRMWARE_VARS_PATH="${candidate}"
      break
    fi
  done
fi

if [ -z "${FIRMWARE_VARS_PATH}" ] || [ ! -f "${FIRMWARE_VARS_PATH}" ]; then
  printf 'missing_qemu_firmware_vars\n' >&2
  exit 1
fi

PROFILE="${PROFILE}" "${ROOT_DIR}/scripts/build-efi.sh"

efi_dir="$(mktemp -d)"
vars_copy="$(mktemp)"
cp "${FIRMWARE_VARS_PATH}" "${vars_copy}"
log_file="$(mktemp)"
semihost_log=""
trap 'rm -rf "${efi_dir}" "${vars_copy}" "${log_file}" "${semihost_log}"' EXIT
mkdir -p "${efi_dir}/EFI/BOOT"
cp "${ROOT_DIR}/target/aarch64-unknown-uefi/${profile_dir}/${EFI_BINARY}.efi" \
  "${efi_dir}/EFI/BOOT/BOOTAA64.EFI"

if [ "${STAGE_NOVA_PAYLOADS}" = "1" ]; then
  if [ "${QEMU_TRACE}" = "1" ] && [ -z "${payload_features}" ]; then
    payload_features="qemu_semihosting"
  fi
  PROFILE="${PROFILE}" PAYLOAD_FEATURES="${payload_features}" "${ROOT_DIR}/scripts/build-kernel.sh" >/dev/null
  mkdir -p "${efi_dir}/nova"
  cp "${ROOT_DIR}/target/aarch64-unknown-none-softfloat/${profile_dir}/stage1-payload.bin" \
    "${efi_dir}/nova/stage1.bin"
  cp "${ROOT_DIR}/target/aarch64-unknown-none-softfloat/${profile_dir}/kernel-payload.bin" \
    "${efi_dir}/nova/kernel.bin"
  PROFILE="${PROFILE}" INITD_FEATURES="${initd_features}" \
    bash "${ROOT_DIR}/scripts/build-init-capsule.sh" "${efi_dir}/nova/init.capsule" >/dev/null
fi

if [ "${QEMU_TRACE}" = "1" ]; then
  semihost_log="$(mktemp)"
  qemu_args+=(
    -chardev "file,id=semihost,path=${semihost_log}"
    -semihosting-config enable=on,target=native,chardev=semihost
  )
fi

set +e
timeout "${TIMEOUT_SECONDS}" qemu-system-aarch64 \
  -machine virt \
  -cpu cortex-a72 \
  -m 2048 \
  -nographic \
  -drive "if=pflash,format=raw,unit=0,readonly=on,file=${FIRMWARE_PATH}" \
  -drive "if=pflash,format=raw,unit=1,file=${vars_copy}" \
  -device qemu-xhci \
  -drive "if=none,file=fat:rw:${efi_dir},format=raw,id=esp" \
  -device usb-storage,drive=esp \
  "${qemu_args[@]}" \
  >"${log_file}" 2>&1
qemu_status=$?
set -e

if [ -n "${semihost_log}" ] && [ -s "${semihost_log}" ]; then
  cat "${semihost_log}" >> "${log_file}"
fi

spark_report_src=""
spark_report_mode=""
for candidate in \
  "${efi_dir}/nova/observatory/spark-observe-report.txt" \
  "${efi_dir}/EFI/BOOT/spark-observe-report.txt"
do
  if [ -f "${candidate}" ]; then
    spark_report_src="${candidate}"
    spark_report_mode="efi_volume"
    break
  fi
done

if [ -z "${spark_report_src}" ] && grep -q 'structured_report_begin' "${log_file}"; then
  stamp="$(date -u +%Y%m%dT%H%M%SZ)"
  report_dir="${ROOT_DIR}/artifacts/reports"
  structured_report_file="${report_dir}/spark-observe-qemu-${stamp}.txt"
  latest_structured_report_link="${report_dir}/latest-spark-observe-report.txt"
  latest_structured_report_path="${report_dir}/latest-spark-observe-path.txt"
  latest_structured_report_status="${report_dir}/latest-spark-observe-status.txt"

  mkdir -p "${report_dir}"
  tr -d '\r' < "${log_file}" \
    | sed -n '/structured_report_begin/,/structured_report_end/p' \
    | sed '/structured_report_begin/d;/structured_report_end/d' \
    > "${structured_report_file}"
  spark_report_mode="console_capture"
elif [ -n "${spark_report_src}" ]; then
  stamp="$(date -u +%Y%m%dT%H%M%SZ)"
  report_dir="${ROOT_DIR}/artifacts/reports"
  structured_report_file="${report_dir}/spark-observe-qemu-${stamp}.txt"
  latest_structured_report_link="${report_dir}/latest-spark-observe-report.txt"
  latest_structured_report_path="${report_dir}/latest-spark-observe-path.txt"
  latest_structured_report_status="${report_dir}/latest-spark-observe-status.txt"

  mkdir -p "${report_dir}"
  cp "${spark_report_src}" "${structured_report_file}"
fi

if [ -n "${spark_report_mode}" ] && [ -s "${structured_report_file}" ]; then
  novaos_refresh_latest_link "${structured_report_file}" "${latest_structured_report_link}"
  novaos_write_latest_path "${structured_report_file}" "${latest_structured_report_path}"
  novaos_write_latest_status "${latest_structured_report_status}" \
    "generated_at_utc=${stamp}" \
    "report_file=${structured_report_file}" \
    "report_mode=${spark_report_mode}" \
    "latest_report_link=${latest_structured_report_link}"

  printf '\nstructured_report=%s\n' "${structured_report_file}" >> "${log_file}"
fi

loader_report_src=""
loader_report_mode=""
if [ "${EFI_BINARY}" = "novaaa64" ]; then
  for candidate in \
    "${efi_dir}/nova/loader/novaaa64-loader-report.txt" \
    "${efi_dir}/EFI/BOOT/novaaa64-loader-report.txt"
  do
    if [ -f "${candidate}" ]; then
      loader_report_src="${candidate}"
      loader_report_mode="efi_volume"
      break
    fi
  done
fi

if [ -z "${loader_report_src}" ] && [ "${EFI_BINARY}" = "novaaa64" ] \
  && grep -q 'loader_handoff_report_begin' "${log_file}"; then
  stamp="$(date -u +%Y%m%dT%H%M%SZ)"
  report_dir="${ROOT_DIR}/artifacts/reports"
  loader_report_file="${report_dir}/novaaa64-qemu-${stamp}.txt"
  latest_loader_report_link="${report_dir}/latest-novaaa64-loader-report.txt"
  latest_loader_report_path="${report_dir}/latest-novaaa64-loader-path.txt"
  latest_loader_report_status="${report_dir}/latest-novaaa64-loader-status.txt"

  mkdir -p "${report_dir}"
  tr -d '\r' < "${log_file}" \
    | sed -n '/loader_handoff_report_begin/,/loader_handoff_report_end/p' \
    | sed '/loader_handoff_report_begin/d;/loader_handoff_report_end/d' \
    > "${loader_report_file}"
  loader_report_mode="console_capture"
elif [ -n "${loader_report_src}" ]; then
  stamp="$(date -u +%Y%m%dT%H%M%SZ)"
  report_dir="${ROOT_DIR}/artifacts/reports"
  loader_report_file="${report_dir}/novaaa64-qemu-${stamp}.txt"
  latest_loader_report_link="${report_dir}/latest-novaaa64-loader-report.txt"
  latest_loader_report_path="${report_dir}/latest-novaaa64-loader-path.txt"
  latest_loader_report_status="${report_dir}/latest-novaaa64-loader-status.txt"

  mkdir -p "${report_dir}"
  cp "${loader_report_src}" "${loader_report_file}"
fi

if [ -n "${loader_report_mode}" ] && [ -s "${loader_report_file}" ]; then
  novaos_refresh_latest_link "${loader_report_file}" "${latest_loader_report_link}"
  novaos_write_latest_path "${loader_report_file}" "${latest_loader_report_path}"
  novaos_write_latest_status "${latest_loader_report_status}" \
    "generated_at_utc=${stamp}" \
    "report_file=${loader_report_file}" \
    "report_mode=${loader_report_mode}" \
    "latest_report_link=${latest_loader_report_link}"

  printf '\nloader_report=%s\n' "${loader_report_file}" >> "${log_file}"
fi

cat "${log_file}"

if [ -n "${EXPECT_TEXTS}" ]; then
  old_ifs="${IFS}"
  IFS=';'
  for pattern in ${EXPECT_TEXTS}; do
    if ! grep -q "${pattern}" "${log_file}"; then
      exit "${qemu_status}"
    fi
  done
  IFS="${old_ifs}"
  exit 0
fi

if grep -q "${EXPECT_TEXT}" "${log_file}"; then
  exit 0
fi

exit "${qemu_status}"
