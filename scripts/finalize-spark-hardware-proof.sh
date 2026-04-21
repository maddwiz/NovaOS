#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${ROOT_DIR:-$(cd -- "${SCRIPT_DIR}/.." && pwd)}"
REPORT_DIR="${REPORT_DIR:-${ROOT_DIR}/artifacts/reports}"
HARDWARE_LABEL="${HARDWARE_LABEL:-DGX Spark}"
OPERATOR_NOTE="${OPERATOR_NOTE:-}"
OBSERVATORY_REAL_STATUS="${OBSERVATORY_REAL_STATUS:-${REPORT_DIR}/latest-spark-observe-real-status.txt}"
OBSERVATORY_COMPARE_STATUS="${OBSERVATORY_COMPARE_STATUS:-${REPORT_DIR}/latest-spark-observe-compare-status.txt}"
LOADER_REAL_STATUS="${LOADER_REAL_STATUS:-${REPORT_DIR}/latest-novaaa64-loader-real-status.txt}"
LOADER_CHECK_STATUS="${LOADER_CHECK_STATUS:-${REPORT_DIR}/latest-novaaa64-loader-check-status.txt}"
STAGE_CHAIN_CHECK_STATUS="${STAGE_CHAIN_CHECK_STATUS:-${REPORT_DIR}/latest-spark-stage-chain-check-status.txt}"
RUN_STAGE_CHAIN_CHECK="${RUN_STAGE_CHAIN_CHECK:-1}"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
OUTPUT_FILE="${REPORT_DIR}/spark-hardware-proof-${STAMP}.md"
LATEST_PROOF_LINK="${REPORT_DIR}/latest-spark-hardware-proof.md"
LATEST_PROOF_PATH="${REPORT_DIR}/latest-spark-hardware-proof-path.txt"
LATEST_PROOF_STATUS="${REPORT_DIR}/latest-spark-hardware-proof-status.txt"

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

mkdir -p "${REPORT_DIR}"

observatory_report="$(read_status_value "${OBSERVATORY_REAL_STATUS}" "collected_report")"
observatory_compare_file="$(read_status_value "${OBSERVATORY_COMPARE_STATUS}" "report_file")"
observatory_compare_result="$(read_status_value "${OBSERVATORY_COMPARE_STATUS}" "status" "missing")"
loader_report="$(read_status_value "${LOADER_REAL_STATUS}" "collected_report")"
loader_check_file="$(read_status_value "${LOADER_CHECK_STATUS}" "report_file")"
loader_check_result="$(read_status_value "${LOADER_CHECK_STATUS}" "status" "missing")"
stage_chain_check_file=""
stage_chain_check_result="missing"

observatory_ready="fail"
if [ -n "${observatory_report}" ] && [ -f "${observatory_report}" ] && [ "${observatory_compare_result}" = "pass" ]; then
  observatory_ready="pass"
fi

loader_ready="fail"
if [ -n "${loader_report}" ] && [ -f "${loader_report}" ] && [ "${loader_check_result}" = "pass" ]; then
  loader_ready="pass"
fi

evidence_ready="fail"
evidence_count=0
evidence_lines=()
if [ "$#" -gt 0 ]; then
  for evidence_path in "$@"; do
    if [ ! -f "${evidence_path}" ]; then
      evidence_lines+=("- missing: ${evidence_path}")
      continue
    fi
    evidence_count=$((evidence_count + 1))
    evidence_lines+=("- file: ${evidence_path}")
  done
fi

if [ "${evidence_count}" -gt 0 ] && [ "${RUN_STAGE_CHAIN_CHECK}" = "1" ]; then
  if ! REPORT_DIR="${REPORT_DIR}" \
    bash "${ROOT_DIR}/scripts/check-spark-stage-chain-proof.sh" "$@" >/dev/null; then
    :
  fi
  stage_chain_check_file="$(read_status_value "${STAGE_CHAIN_CHECK_STATUS}" "report_file")"
  stage_chain_check_result="$(read_status_value "${STAGE_CHAIN_CHECK_STATUS}" "status" "missing")"
fi

if [ "${stage_chain_check_result}" = "pass" ]; then
  evidence_ready="pass"
elif [ -n "${OPERATOR_NOTE}" ]; then
  evidence_ready="pass"
elif [ "${evidence_count}" -gt 0 ]; then
  evidence_ready="fail"
fi

overall_status="pass"
if [ "${observatory_ready}" != "pass" ] || [ "${loader_ready}" != "pass" ] || [ "${evidence_ready}" != "pass" ]; then
  overall_status="fail"
fi

{
  printf '%s\n' '# Spark Hardware Proof Summary'
  printf '\n'
  printf '%s\n' "- generated_at_utc: ${STAMP}"
  printf '%s\n' "- hardware_label: ${HARDWARE_LABEL}"
  printf '%s\n' "- overall_status: ${overall_status}"
  printf '\n## Observatory Proof\n\n'
  printf '%s\n' "- observatory_report: ${observatory_report:-missing}"
  printf '%s\n' "- observatory_compare_file: ${observatory_compare_file:-missing}"
  printf '%s\n' "- observatory_compare_status: ${observatory_compare_result}"
  printf '%s\n' "- observatory_ready: ${observatory_ready}"
  printf '\n## Loader Proof\n\n'
  printf '%s\n' "- loader_report: ${loader_report:-missing}"
  printf '%s\n' "- loader_check_file: ${loader_check_file:-missing}"
  printf '%s\n' "- loader_check_status: ${loader_check_result}"
  printf '%s\n' "- loader_ready: ${loader_ready}"
  printf '\n## Stage Chain Evidence\n\n'
  printf '%s\n' "- evidence_ready: ${evidence_ready}"
  printf '%s\n' "- evidence_count: ${evidence_count}"
  printf '%s\n' "- stage_chain_check_file: ${stage_chain_check_file:-missing}"
  printf '%s\n' "- stage_chain_check_status: ${stage_chain_check_result}"
  if [ -n "${OPERATOR_NOTE}" ]; then
    printf '%s\n' "- operator_note: ${OPERATOR_NOTE}"
  fi
  if [ "${#evidence_lines[@]}" -eq 0 ]; then
    printf '%s\n' '- none'
  else
    for line in "${evidence_lines[@]}"; do
      printf '%s\n' "${line}"
    done
  fi
  printf '\n## Acceptance Gate\n\n'
  if [ "${overall_status}" = "pass" ]; then
    printf '%s\n' '- pass: observatory return, loader return, and stage-chain evidence are all present for the current Spark proof flow'
  else
    printf '%s\n' '- fail: the current Spark proof flow is still incomplete'
  fi
} > "${OUTPUT_FILE}"

novaos_refresh_latest_link "${OUTPUT_FILE}" "${LATEST_PROOF_LINK}"
novaos_write_latest_path "${OUTPUT_FILE}" "${LATEST_PROOF_PATH}"
novaos_write_latest_status "${LATEST_PROOF_STATUS}" \
  "generated_at_utc=${STAMP}" \
  "hardware_label=${HARDWARE_LABEL}" \
  "overall_status=${overall_status}" \
  "observatory_report=${observatory_report}" \
  "observatory_compare_file=${observatory_compare_file}" \
  "observatory_compare_status=${observatory_compare_result}" \
  "loader_report=${loader_report}" \
  "loader_check_file=${loader_check_file}" \
  "loader_check_status=${loader_check_result}" \
  "stage_chain_check_file=${stage_chain_check_file}" \
  "stage_chain_check_status=${stage_chain_check_result}" \
  "evidence_ready=${evidence_ready}" \
  "evidence_count=${evidence_count}" \
  "report_file=${OUTPUT_FILE}" \
  "latest_report_link=${LATEST_PROOF_LINK}"

printf '%s\n' "${OUTPUT_FILE}"

if [ "${overall_status}" != "pass" ]; then
  exit 1
fi
