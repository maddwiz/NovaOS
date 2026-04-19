#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${ROOT_DIR:-/home/nova/NovaOS}"
REPORT_DIR="${REPORT_DIR:-${ROOT_DIR}/artifacts/reports}"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
OUTPUT_FILE="${OUTPUT_FILE:-${REPORT_DIR}/spark-stage-chain-check-${STAMP}.md}"
LATEST_CHECK_LINK="${REPORT_DIR}/latest-spark-stage-chain-check.md"
LATEST_CHECK_PATH="${REPORT_DIR}/latest-spark-stage-chain-check-path.txt"
LATEST_CHECK_STATUS="${REPORT_DIR}/latest-spark-stage-chain-check-status.txt"

source "${ROOT_DIR}/scripts/novaos-latest.sh"

if [ "$#" -eq 0 ]; then
  printf 'usage: %s EVIDENCE_FILE [EVIDENCE_FILE ...]\n' "$0" >&2
  exit 64
fi

mkdir -p "${REPORT_DIR}"

declare -a evidence_lines=()
declare -a existing_files=()
declare -a contract_failures=()
declare -a contract_lines=()

for evidence_path in "$@"; do
  if [ ! -f "${evidence_path}" ]; then
    evidence_lines+=("- missing: ${evidence_path}")
    contract_failures+=("missing evidence file: ${evidence_path}")
    continue
  fi

  existing_files+=("${evidence_path}")
  evidence_lines+=("- file: ${evidence_path}")
done

MATCHED_PATTERN=""
MATCHED_FILE=""
combined_evidence_file=""

match_marker_group() {
  local pattern file

  MATCHED_PATTERN=""
  MATCHED_FILE=""

  for pattern in "$@"; do
    for file in "${existing_files[@]}"; do
      if grep -Fq -- "${pattern}" "${file}"; then
        MATCHED_PATTERN="${pattern}"
        MATCHED_FILE="${file}"
        return 0
      fi
    done
  done

  return 1
}

line_for_pattern() {
  local pattern="$1"

  if [ -z "${combined_evidence_file}" ] || [ ! -f "${combined_evidence_file}" ]; then
    return 1
  fi

  grep -nF -- "${pattern}" "${combined_evidence_file}" | head -n 1 | cut -d: -f1
}

stage0_status="fail"
stage0_pattern=""
stage0_file=""
if [ "${#existing_files[@]}" -eq 0 ]; then
  contract_failures+=("no evidence files were present")
elif match_marker_group \
  "NovaOS stage0 loader" \
  "NovaOS stage0 pre-exit" \
  "NovaOS stage0 post-exit"; then
  stage0_status="pass"
  stage0_pattern="${MATCHED_PATTERN}"
  stage0_file="${MATCHED_FILE}"
  contract_lines+=("- pass: stage0 marker matched ${stage0_pattern} in ${stage0_file}")
else
  contract_failures+=("missing stage0 marker (expected loader, pre-exit, or post-exit)")
fi

stage1_status="fail"
stage1_pattern=""
stage1_file=""
if match_marker_group "NovaOS stage1 entered"; then
  stage1_status="pass"
  stage1_pattern="${MATCHED_PATTERN}"
  stage1_file="${MATCHED_FILE}"
  contract_lines+=("- pass: stage1 marker matched ${stage1_pattern} in ${stage1_file}")
else
  contract_failures+=("missing stage1 marker (expected NovaOS stage1 entered)")
fi

bootinfo_v2_status="fail"
bootinfo_v2_pattern=""
bootinfo_v2_file=""
if match_marker_group "NovaOS stage1 bootinfo_v2 sidecar"; then
  bootinfo_v2_status="pass"
  bootinfo_v2_pattern="${MATCHED_PATTERN}"
  bootinfo_v2_file="${MATCHED_FILE}"
  contract_lines+=("- pass: BootInfo v2 sidecar marker matched ${bootinfo_v2_pattern} in ${bootinfo_v2_file}")
else
  contract_failures+=(
    "missing BootInfo v2 sidecar marker (expected NovaOS stage1 bootinfo_v2 sidecar)"
  )
fi

kernel_status="fail"
kernel_pattern=""
kernel_file=""
if match_marker_group "NovaOS kernel entered"; then
  kernel_status="pass"
  kernel_pattern="${MATCHED_PATTERN}"
  kernel_file="${MATCHED_FILE}"
  contract_lines+=("- pass: kernel marker matched ${kernel_pattern} in ${kernel_file}")
else
  contract_failures+=("missing kernel marker (expected NovaOS kernel entered)")
fi

if [ "${#existing_files[@]}" -gt 0 ]; then
  combined_evidence_file="$(mktemp)"
  trap 'rm -f "${combined_evidence_file}"' EXIT

  for evidence_file in "${existing_files[@]}"; do
    cat "${evidence_file}" >> "${combined_evidence_file}"
    printf '\n' >> "${combined_evidence_file}"
  done
fi

sequence_status="fail"
sequence_detail=""
stage0_line="$(line_for_pattern "${stage0_pattern:-missing}" || true)"
stage1_line="$(line_for_pattern "NovaOS stage1 entered" || true)"
bootinfo_v2_line="$(line_for_pattern "NovaOS stage1 bootinfo_v2 sidecar" || true)"
kernel_line="$(line_for_pattern "NovaOS kernel entered" || true)"
bootinfo_v2_absent_line="$(line_for_pattern "NovaOS stage1 bootinfo_v2 absent" || true)"
bootinfo_v2_absent_status="pass"

if [ -n "${bootinfo_v2_absent_line}" ]; then
  bootinfo_v2_absent_status="fail"
  contract_failures+=(
    "unexpected BootInfo v2 absent marker at line ${bootinfo_v2_absent_line}"
  )
fi

if [ -n "${stage0_line}" ] && [ -n "${stage1_line}" ] \
  && [ -n "${bootinfo_v2_line}" ] && [ -n "${kernel_line}" ] \
  && [ "${stage0_line}" -lt "${stage1_line}" ] \
  && [ "${stage1_line}" -lt "${bootinfo_v2_line}" ] \
  && [ "${bootinfo_v2_line}" -lt "${kernel_line}" ]; then
  sequence_status="pass"
  sequence_detail="stage0=${stage0_line},stage1=${stage1_line},bootinfo_v2=${bootinfo_v2_line},kernel=${kernel_line}"
  contract_lines+=("- pass: stage-chain markers appear in order (${sequence_detail})")
else
  contract_failures+=(
    "stage-chain markers are not in order (stage0=${stage0_line:-missing}, stage1=${stage1_line:-missing}, bootinfo_v2=${bootinfo_v2_line:-missing}, kernel=${kernel_line:-missing})"
  )
fi

status="pass"
if [ "${#contract_failures[@]}" -ne 0 ]; then
  status="fail"
fi

{
  printf '%s\n' '# Spark Stage Chain Check'
  printf '\n'
  printf '%s\n' "- generated_at_utc: ${STAMP}"
  printf '%s\n' "- status: ${status}"
  printf '%s\n' "- evidence_file_count: ${#existing_files[@]}"
  printf '\n## Evidence Files\n\n'
  for line in "${evidence_lines[@]}"; do
    printf '%s\n' "${line}"
  done
  printf '\n## Contract Checks\n\n'
  if [ "${#contract_failures[@]}" -eq 0 ]; then
    for line in "${contract_lines[@]}"; do
      printf '%s\n' "${line}"
    done
  else
    for failure in "${contract_failures[@]}"; do
      printf '%s\n' "- fail: ${failure}"
    done
  fi
} > "${OUTPUT_FILE}"

novaos_refresh_latest_link "${OUTPUT_FILE}" "${LATEST_CHECK_LINK}"
novaos_write_latest_path "${OUTPUT_FILE}" "${LATEST_CHECK_PATH}"
novaos_write_latest_status "${LATEST_CHECK_STATUS}" \
  "generated_at_utc=${STAMP}" \
  "status=${status}" \
  "evidence_file_count=${#existing_files[@]}" \
  "stage0_marker_status=${stage0_status}" \
  "stage0_marker_pattern=${stage0_pattern}" \
  "stage0_marker_file=${stage0_file}" \
  "stage1_marker_status=${stage1_status}" \
  "stage1_marker_pattern=${stage1_pattern}" \
  "stage1_marker_file=${stage1_file}" \
  "bootinfo_v2_marker_status=${bootinfo_v2_status}" \
  "bootinfo_v2_marker_pattern=${bootinfo_v2_pattern}" \
  "bootinfo_v2_marker_file=${bootinfo_v2_file}" \
  "bootinfo_v2_absent_status=${bootinfo_v2_absent_status}" \
  "kernel_marker_status=${kernel_status}" \
  "kernel_marker_pattern=${kernel_pattern}" \
  "kernel_marker_file=${kernel_file}" \
  "sequence_status=${sequence_status}" \
  "sequence_detail=${sequence_detail}" \
  "report_file=${OUTPUT_FILE}" \
  "latest_report_link=${LATEST_CHECK_LINK}"

printf '%s\n' "${OUTPUT_FILE}"

if [ "${status}" != "pass" ]; then
  exit 1
fi
