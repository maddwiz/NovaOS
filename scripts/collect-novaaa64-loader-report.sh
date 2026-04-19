#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${ROOT_DIR:-/home/nova/NovaOS}"
REPORT_DIR="${REPORT_DIR:-${ROOT_DIR}/artifacts/reports}"
ESP_MOUNT="${ESP_MOUNT:-/boot/efi}"
REPORT_SOURCE="${REPORT_SOURCE:-${1:-}}"
BASELINE_REPORT="${BASELINE_REPORT:-${ROOT_DIR}/artifacts/reports/latest-novaaa64-loader-report.txt}"
RUN_CHECK="${RUN_CHECK:-1}"
export PATH="/home/linuxbrew/.linuxbrew/bin:/home/nova/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:${PATH:-}"
source "${ROOT_DIR}/scripts/novaos-latest.sh"

if [ -z "${REPORT_SOURCE}" ]; then
  for candidate in \
    "${ESP_MOUNT}/nova/loader/novaaa64-loader-report.txt" \
    "${ESP_MOUNT}/EFI/BOOT/novaaa64-loader-report.txt"
  do
    if [ -f "${candidate}" ]; then
      REPORT_SOURCE="${candidate}"
      break
    fi
  done
fi

if [ -z "${REPORT_SOURCE}" ] || [ ! -f "${REPORT_SOURCE}" ]; then
  printf 'missing_report_source=%s\n' "${REPORT_SOURCE:-unset}" >&2
  exit 1
fi

stamp="$(date -u +%Y%m%dT%H%M%SZ)"
copied_report="${REPORT_DIR}/novaaa64-loader-real-${stamp}.txt"
latest_link="${REPORT_DIR}/latest-novaaa64-loader-real-report.txt"
path_file="${REPORT_DIR}/latest-novaaa64-loader-real-path.txt"
status_file="${REPORT_DIR}/latest-novaaa64-loader-real-status.txt"

mkdir -p "${REPORT_DIR}"
tr -d '\r' < "${REPORT_SOURCE}" > "${copied_report}"

grep -q '^report_kind=novaaa64_loader_handoff_report$' "${copied_report}"
grep -q '^report_version=1$' "${copied_report}"
grep -q '^stage1_plan_ready=' "${copied_report}"
grep -q '^boot_info_v2_valid=' "${copied_report}"

check_file=""
if [ "${RUN_CHECK}" = "1" ]; then
  check_file="$(bash "${ROOT_DIR}/scripts/check-novaaa64-loader-report.sh" \
    "${copied_report}" \
    "${BASELINE_REPORT}")"
fi

novaos_refresh_latest_link "${copied_report}" "${latest_link}"
novaos_write_latest_path "${copied_report}" "${path_file}"
novaos_write_latest_status "${status_file}" \
  "generated_at_utc=${stamp}" \
  "source_report=${REPORT_SOURCE}" \
  "collected_report=${copied_report}" \
  "baseline_report=${BASELINE_REPORT}" \
  "check_ran=${RUN_CHECK}" \
  "check_file=${check_file}" \
  "latest_report_link=${latest_link}"

printf 'source_report=%s\n' "${REPORT_SOURCE}"
printf 'collected_report=%s\n' "${copied_report}"
printf 'baseline_report=%s\n' "${BASELINE_REPORT}"
printf 'check_ran=%s\n' "${RUN_CHECK}"
if [ -n "${check_file}" ]; then
  printf 'check_file=%s\n' "${check_file}"
fi
