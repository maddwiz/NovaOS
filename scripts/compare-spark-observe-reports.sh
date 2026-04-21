#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${ROOT_DIR:-$(cd -- "${SCRIPT_DIR}/.." && pwd)}"
REPORT_DIR="${REPORT_DIR:-${ROOT_DIR}/artifacts/reports}"
BASELINE_REPORT_DEFAULT="${REPORT_DIR}/latest-spark-observe-report.txt"
REAL_REPORT="${1:-}"
BASELINE_REPORT="${2:-${BASELINE_REPORT_DEFAULT}}"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
OUTPUT_FILE="${OUTPUT_FILE:-${REPORT_DIR}/spark-observe-compare-${STAMP}.md}"
LATEST_COMPARE_LINK="${REPORT_DIR}/latest-spark-observe-compare.md"
LATEST_COMPARE_PATH="${REPORT_DIR}/latest-spark-observe-compare-path.txt"
LATEST_COMPARE_STATUS="${REPORT_DIR}/latest-spark-observe-compare-status.txt"

source "${ROOT_DIR}/scripts/novaos-latest.sh"

if [ -z "${REAL_REPORT}" ]; then
  printf 'usage: %s REAL_REPORT [BASELINE_REPORT]\n' "$0" >&2
  exit 64
fi

if [ ! -f "${REAL_REPORT}" ]; then
  printf 'missing_real_report=%s\n' "${REAL_REPORT}" >&2
  exit 1
fi

if [ ! -f "${BASELINE_REPORT}" ]; then
  printf 'missing_baseline_report=%s\n' "${BASELINE_REPORT}" >&2
  exit 1
fi

mkdir -p "${REPORT_DIR}"

declare -A baseline_map=()
declare -A real_map=()
contract_failures=()
diff_lines=()

load_report() {
  local file_path="$1"
  local -n out_map="$2"
  local line key value

  while IFS= read -r line; do
    if [[ -z "${line}" || "${line}" != *=* ]]; then
      continue
    fi
    key="${line%%=*}"
    value="${line#*=}"
    out_map["${key}"]="${value}"
  done < "${file_path}"
}

load_report "${BASELINE_REPORT}" baseline_map
load_report "${REAL_REPORT}" real_map

check_equals() {
  local key="$1"
  local expected="$2"
  local actual="${real_map[${key}]:-}"

  if [ "${actual}" != "${expected}" ]; then
    contract_failures+=("${key}: expected ${expected}, got ${actual:-missing}")
  fi
}

check_present() {
  local key="$1"
  if [ -z "${real_map[${key}]+x}" ]; then
    contract_failures+=("${key}: missing")
  fi
}

check_nonzero_count() {
  local key="$1"
  local value="${real_map[${key}]:-}"

  if [ -z "${value}" ]; then
    contract_failures+=("${key}: missing")
    return
  fi

  if ! [[ "${value}" =~ ^[0-9]+$ ]]; then
    contract_failures+=("${key}: not_numeric (${value})")
    return
  fi

  if [ "${value}" -eq 0 ]; then
    contract_failures+=("${key}: expected non-zero")
  fi
}

compare_note() {
  local key="$1"
  local baseline_value="${baseline_map[${key}]:-missing}"
  local real_value="${real_map[${key}]:-missing}"

  if [ "${baseline_value}" != "${real_value}" ]; then
    diff_lines+=("- ${key}: qemu=${baseline_value} real=${real_value}")
  fi
}

check_equals "report_kind" "spark_observatory_v2_seed_report"
check_equals "report_version" "1"
check_present "display_seed_count"
check_nonzero_count "storage_seed_count"
check_present "network_seed_count"
check_nonzero_count "accel_seed_draft_count"
check_equals "accel_seed_draft[0].transport" "integrated"
check_equals "accel_seed_draft[0].topology_hint" "uma"
check_equals "accel_seed_draft[0].memory_topology" "uma"
check_equals "accel_seed_draft[0].platform_ready" "true"

for key in \
  firmware_vendor \
  firmware_revision \
  loaded_image_path \
  loaded_image_path_known \
  secure_boot_enabled \
  setup_mode \
  acpi_rsdp \
  dtb \
  smbios \
  framebuffer \
  display_seed_count \
  storage_seed_count \
  network_seed_count
do
  compare_note "${key}"
done

status="pass"
if [ "${#contract_failures[@]}" -ne 0 ]; then
  status="fail"
fi

{
  printf '%s\n' '# Spark Observatory Comparison'
  printf '\n'
  printf '%s\n' "- generated_at_utc: ${STAMP}"
  printf '%s\n' "- status: ${status}"
  printf '%s\n' "- real_report: ${REAL_REPORT}"
  printf '%s\n' "- baseline_report: ${BASELINE_REPORT}"
  printf '\n## Contract Checks\n\n'
  if [ "${#contract_failures[@]}" -eq 0 ]; then
    printf '%s\n' '- pass: candidate report satisfies the current BootInfo v2 observatory contract checks'
  else
    for failure in "${contract_failures[@]}"; do
      printf '%s\n' "- fail: ${failure}"
    done
  fi
  printf '\n## Differences From QEMU Baseline\n\n'
  if [ "${#diff_lines[@]}" -eq 0 ]; then
    printf '%s\n' '- none'
  else
    for line in "${diff_lines[@]}"; do
      printf '%s\n' "${line}"
    done
  fi
} > "${OUTPUT_FILE}"

novaos_refresh_latest_link "${OUTPUT_FILE}" "${LATEST_COMPARE_LINK}"
novaos_write_latest_path "${OUTPUT_FILE}" "${LATEST_COMPARE_PATH}"
novaos_write_latest_status "${LATEST_COMPARE_STATUS}" \
  "generated_at_utc=${STAMP}" \
  "status=${status}" \
  "real_report=${REAL_REPORT}" \
  "baseline_report=${BASELINE_REPORT}" \
  "report_file=${OUTPUT_FILE}" \
  "latest_report_link=${LATEST_COMPARE_LINK}"

printf '%s\n' "${OUTPUT_FILE}"

if [ "${status}" != "pass" ]; then
  exit 1
fi
