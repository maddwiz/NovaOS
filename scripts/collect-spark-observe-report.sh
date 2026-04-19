#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${ROOT_DIR:-/home/nova/NovaOS}"
REPORT_DIR="${REPORT_DIR:-${ROOT_DIR}/artifacts/reports}"
ESP_MOUNT="${ESP_MOUNT:-/boot/efi}"
REPORT_SOURCE="${REPORT_SOURCE:-${1:-}}"
BASELINE_REPORT="${BASELINE_REPORT:-${ROOT_DIR}/artifacts/reports/latest-spark-observe-report.txt}"
RUN_COMPARE="${RUN_COMPARE:-1}"
export PATH="/home/linuxbrew/.linuxbrew/bin:/home/nova/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:${PATH:-}"
source "${ROOT_DIR}/scripts/novaos-latest.sh"

if [ -z "${REPORT_SOURCE}" ]; then
  for candidate in \
    "${ESP_MOUNT}/nova/observatory/spark-observe-report.txt" \
    "${ESP_MOUNT}/EFI/BOOT/spark-observe-report.txt"
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
copied_report="${REPORT_DIR}/spark-observe-real-${stamp}.txt"
latest_link="${REPORT_DIR}/latest-spark-observe-real-report.txt"
path_file="${REPORT_DIR}/latest-spark-observe-real-path.txt"
status_file="${REPORT_DIR}/latest-spark-observe-real-status.txt"

mkdir -p "${REPORT_DIR}"
tr -d '\r' < "${REPORT_SOURCE}" > "${copied_report}"

grep -q '^report_kind=spark_observatory_v2_seed_report$' "${copied_report}"
grep -q '^display_seed_count=' "${copied_report}"
grep -q '^storage_seed_count=' "${copied_report}"
grep -q '^network_seed_count=' "${copied_report}"
grep -q '^accel_seed_draft_count=' "${copied_report}"

compare_file=""
if [ "${RUN_COMPARE}" = "1" ]; then
  compare_file="$("${ROOT_DIR}/scripts/compare-spark-observe-reports.sh" \
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
  "compare_ran=${RUN_COMPARE}" \
  "compare_file=${compare_file}" \
  "latest_report_link=${latest_link}"

printf 'source_report=%s\n' "${REPORT_SOURCE}"
printf 'collected_report=%s\n' "${copied_report}"
printf 'baseline_report=%s\n' "${BASELINE_REPORT}"
printf 'compare_ran=%s\n' "${RUN_COMPARE}"
if [ -n "${compare_file}" ]; then
  printf 'compare_file=%s\n' "${compare_file}"
fi
