#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${ROOT_DIR:-$(cd -- "${SCRIPT_DIR}/.." && pwd)}"
REPORT_DIR="${REPORT_DIR:-${ROOT_DIR}/artifacts/reports}"
ESP_MOUNT="${ESP_MOUNT:-/boot/efi}"
OBSERVATORY_SOURCE="${OBSERVATORY_SOURCE:-}"
LOADER_SOURCE="${LOADER_SOURCE:-}"
OBS_BASELINE_REPORT="${OBS_BASELINE_REPORT:-${ROOT_DIR}/artifacts/reports/latest-spark-observe-report.txt}"
LOADER_BASELINE_REPORT="${LOADER_BASELINE_REPORT:-${ROOT_DIR}/artifacts/reports/latest-novaaa64-loader-report.txt}"
RUN_COMPARE="${RUN_COMPARE:-1}"
RUN_CHECK="${RUN_CHECK:-1}"
HARDWARE_LABEL="${HARDWARE_LABEL:-DGX Spark}"
OPERATOR_NOTE="${OPERATOR_NOTE:-}"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
LATEST_COMPLETE_STATUS="${REPORT_DIR}/latest-spark-hardware-proof-complete-status.txt"
LATEST_COMPLETE_PATH="${REPORT_DIR}/latest-spark-hardware-proof-complete-path.txt"

source "${ROOT_DIR}/scripts/novaos-latest.sh"

read_status_value() {
  local file_path="$1"
  local key="$2"
  local default_value="${3:-}"
  local value

  if [ ! -f "${file_path}" ]; then
    printf '%s\n' "${default_value}"
    return
  fi

  value="$(grep -E "^${key}=" "${file_path}" | tail -n 1 | cut -d= -f2- || true)"
  if [ -z "${value}" ]; then
    printf '%s\n' "${default_value}"
  else
    printf '%s\n' "${value}"
  fi
}

if [ "$#" -eq 0 ] && [ -z "${OPERATOR_NOTE}" ]; then
  printf 'missing_stage_chain_evidence\n' >&2
  exit 64
fi

mkdir -p "${REPORT_DIR}"

if [ -n "${OBSERVATORY_SOURCE}" ]; then
  REPORT_DIR="${REPORT_DIR}" \
  ESP_MOUNT="${ESP_MOUNT}" \
  BASELINE_REPORT="${OBS_BASELINE_REPORT}" \
  RUN_COMPARE="${RUN_COMPARE}" \
    bash "${ROOT_DIR}/scripts/collect-spark-observe-report.sh" "${OBSERVATORY_SOURCE}" >/dev/null
else
  REPORT_DIR="${REPORT_DIR}" \
  ESP_MOUNT="${ESP_MOUNT}" \
  BASELINE_REPORT="${OBS_BASELINE_REPORT}" \
  RUN_COMPARE="${RUN_COMPARE}" \
    bash "${ROOT_DIR}/scripts/collect-spark-observe-report.sh" >/dev/null
fi

if [ -n "${LOADER_SOURCE}" ]; then
  REPORT_DIR="${REPORT_DIR}" \
  ESP_MOUNT="${ESP_MOUNT}" \
  BASELINE_REPORT="${LOADER_BASELINE_REPORT}" \
  RUN_CHECK="${RUN_CHECK}" \
    bash "${ROOT_DIR}/scripts/collect-novaaa64-loader-report.sh" "${LOADER_SOURCE}" >/dev/null
else
  REPORT_DIR="${REPORT_DIR}" \
  ESP_MOUNT="${ESP_MOUNT}" \
  BASELINE_REPORT="${LOADER_BASELINE_REPORT}" \
  RUN_CHECK="${RUN_CHECK}" \
    bash "${ROOT_DIR}/scripts/collect-novaaa64-loader-report.sh" >/dev/null
fi

final_report="$(
  REPORT_DIR="${REPORT_DIR}" \
  HARDWARE_LABEL="${HARDWARE_LABEL}" \
  OPERATOR_NOTE="${OPERATOR_NOTE}" \
  OBSERVATORY_REAL_STATUS="${REPORT_DIR}/latest-spark-observe-real-status.txt" \
  OBSERVATORY_COMPARE_STATUS="${REPORT_DIR}/latest-spark-observe-compare-status.txt" \
  LOADER_REAL_STATUS="${REPORT_DIR}/latest-novaaa64-loader-real-status.txt" \
  LOADER_CHECK_STATUS="${REPORT_DIR}/latest-novaaa64-loader-check-status.txt" \
    bash "${ROOT_DIR}/scripts/finalize-spark-hardware-proof.sh" "$@"
)"

overall_status="$(read_status_value "${REPORT_DIR}/latest-spark-hardware-proof-status.txt" "overall_status" "missing")"

novaos_write_latest_path "${final_report}" "${LATEST_COMPLETE_PATH}"
novaos_write_latest_status "${LATEST_COMPLETE_STATUS}" \
  "generated_at_utc=${STAMP}" \
  "report_dir=${REPORT_DIR}" \
  "esp_mount=${ESP_MOUNT}" \
  "observatory_source=${OBSERVATORY_SOURCE:-auto}" \
  "loader_source=${LOADER_SOURCE:-auto}" \
  "run_compare=${RUN_COMPARE}" \
  "run_check=${RUN_CHECK}" \
  "hardware_label=${HARDWARE_LABEL}" \
  "operator_note=${OPERATOR_NOTE}" \
  "evidence_count=$#" \
  "overall_status=${overall_status}" \
  "final_report=${final_report}" \
  "observatory_real_status=${REPORT_DIR}/latest-spark-observe-real-status.txt" \
  "observatory_compare_status=${REPORT_DIR}/latest-spark-observe-compare-status.txt" \
  "loader_real_status=${REPORT_DIR}/latest-novaaa64-loader-real-status.txt" \
  "loader_check_status=${REPORT_DIR}/latest-novaaa64-loader-check-status.txt" \
  "stage_chain_check_status=${REPORT_DIR}/latest-spark-stage-chain-check-status.txt" \
  "final_status=${REPORT_DIR}/latest-spark-hardware-proof-status.txt"

printf '%s\n' "${final_report}"
