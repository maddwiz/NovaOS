#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${ROOT_DIR:-$(cd -- "${SCRIPT_DIR}/.." && pwd)}"
REPORT_DIR="${REPORT_DIR:-${ROOT_DIR}/artifacts/reports}"
BASELINE_REPORT_DEFAULT="${REPORT_DIR}/latest-novaaa64-loader-report.txt"
REAL_REPORT="${1:-}"
BASELINE_REPORT="${2:-${BASELINE_REPORT_DEFAULT}}"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
OUTPUT_FILE="${OUTPUT_FILE:-${REPORT_DIR}/novaaa64-loader-check-${STAMP}.md}"
LATEST_CHECK_LINK="${REPORT_DIR}/latest-novaaa64-loader-check.md"
LATEST_CHECK_PATH="${REPORT_DIR}/latest-novaaa64-loader-check-path.txt"
LATEST_CHECK_STATUS="${REPORT_DIR}/latest-novaaa64-loader-check-status.txt"

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

load_report "${BASELINE_REPORT}" baseline_map
load_report "${REAL_REPORT}" real_map

check_equals "report_kind" "novaaa64_loader_handoff_report"
check_equals "report_version" "1"
check_equals "stage1_plan_ready" "true"
check_equals "boot_info_valid" "true"
check_equals "boot_info_v2_valid" "true"
check_equals "boot_info_v2.present" "true"
check_equals "boot_info_v2_bootstrap_payload_present" "true"
check_equals "boot_info_v2_bootstrap_user_window_present" "true"
check_equals "boot_info_v2_bootstrap_frame_arena_present" "true"
check_equals "bootstrap_frame_arena_v2.present" "true"
check_equals "stage1_plan.present" "true"
check_equals "stage1_image.present" "true"
check_equals "kernel_image.present" "true"
check_equals "init_capsule.present" "true"
check_equals "loader_log.present" "true"
check_nonzero_count "boot_info_v2_storage_seed_count"
check_present "boot_info_v2_network_seed_count"
check_nonzero_count "boot_info_v2_accel_seed_count"
check_nonzero_count "boot_info_v2_bootstrap_payload_size"
check_present "boot_info_v2_bootstrap_user_window_base"
check_nonzero_count "boot_info_v2_bootstrap_user_window_size"
check_nonzero_count "boot_info_v2_bootstrap_user_stack_size"
check_present "boot_info_v2_bootstrap_frame_arena_base"
check_nonzero_count "boot_info_v2_bootstrap_frame_arena_size"
check_present "boot_info_v2_platform_class"
check_present "boot_info_v2_memory_topology"

for key in \
  stage1_plan_ready \
  boot_info_valid \
  boot_info_v2_valid \
  boot_info_v2_platform_class \
  boot_info_v2_memory_topology \
  boot_info_v2_storage_seed_count \
  boot_info_v2_network_seed_count \
  boot_info_v2_accel_seed_count \
  boot_info_v2_bootstrap_payload_present \
  boot_info_v2_bootstrap_payload_size \
  boot_info_v2_bootstrap_user_window_present \
  boot_info_v2_bootstrap_user_window_base \
  boot_info_v2_bootstrap_user_window_size \
  boot_info_v2_bootstrap_user_stack_size \
  boot_info_v2_bootstrap_frame_arena_present \
  boot_info_v2_bootstrap_frame_arena_base \
  boot_info_v2_bootstrap_frame_arena_size \
  firmware_revision \
  secure_boot_state \
  boot_source \
  current_el \
  config_table_count \
  memory_map_entries \
  framebuffer_present
do
  compare_note "${key}"
done

status="pass"
if [ "${#contract_failures[@]}" -ne 0 ]; then
  status="fail"
fi

{
  printf '%s\n' '# NovaAA64 Loader Report Check'
  printf '\n'
  printf '%s\n' "- generated_at_utc: ${STAMP}"
  printf '%s\n' "- status: ${status}"
  printf '%s\n' "- real_report: ${REAL_REPORT}"
  printf '%s\n' "- baseline_report: ${BASELINE_REPORT}"
  printf '\n## Contract Checks\n\n'
  if [ "${#contract_failures[@]}" -eq 0 ]; then
    printf '%s\n' '- pass: candidate loader handoff report satisfies the current Spark loader return-path checks'
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

novaos_refresh_latest_link "${OUTPUT_FILE}" "${LATEST_CHECK_LINK}"
novaos_write_latest_path "${OUTPUT_FILE}" "${LATEST_CHECK_PATH}"
novaos_write_latest_status "${LATEST_CHECK_STATUS}" \
  "generated_at_utc=${STAMP}" \
  "status=${status}" \
  "real_report=${REAL_REPORT}" \
  "baseline_report=${BASELINE_REPORT}" \
  "report_file=${OUTPUT_FILE}" \
  "latest_report_link=${LATEST_CHECK_LINK}"

printf '%s\n' "${OUTPUT_FILE}"

if [ "${status}" != "pass" ]; then
  exit 1
fi
