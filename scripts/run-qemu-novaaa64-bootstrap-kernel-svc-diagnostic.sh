#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${ROOT_DIR:-/home/nova/NovaOS}"
REPORT_DIR="${REPORT_DIR:-${ROOT_DIR}/artifacts/reports}"
TIMEOUT_SECONDS="${TIMEOUT_SECONDS:-12}"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
LOG_FILE="${REPORT_DIR}/bootstrap-kernel-svc-diagnostic-${STAMP}.log"
LATEST_LOG_LINK="${REPORT_DIR}/latest-bootstrap-kernel-svc-diagnostic.log"
LATEST_LOG_PATH="${REPORT_DIR}/latest-bootstrap-kernel-svc-diagnostic-path.txt"
LATEST_STATUS_FILE="${REPORT_DIR}/latest-bootstrap-kernel-svc-diagnostic-status.txt"

source "${ROOT_DIR}/scripts/novaos-latest.sh"

mkdir -p "${REPORT_DIR}"

set +e
EFI_BINARY=novaaa64 \
EXPECT_TEXTS="NovaOS stage0 loader;NovaOS stage1 entered;NovaOS stage1 bootinfo_v2 sidecar;NovaOS kernel entered;NovaOS kernel bring-up;init capsule summary observed;bootstrap task current initd;bootstrap task launch plan from bootinfo_v2;bootstrap capability probe passed;bootstrap endpoint probe passed;NovaOS bootstrap kernel svc begin" \
STAGE_NOVA_PAYLOADS=1 \
PAYLOAD_FEATURES=qemu_virt_trace,bootstrap_trap_vector_trace,bootstrap_kernel_svc_probe \
INITD_FEATURES=qemu_virt_trace \
NOVAAA64_FEATURES=qemu_virt_trace \
TIMEOUT_SECONDS="${TIMEOUT_SECONDS}" \
  "${ROOT_DIR}/scripts/run-qemu-spark-observe.sh" > "${LOG_FILE}" 2>&1
runner_status=$?
set -e

cat "${LOG_FILE}"

kernel_begin_seen=false
kernel_success_seen=false
kernel_failure_seen=false
caller_capture_matched=false
caller_capture_mismatch=false
vector_prestack_seen=false
vector_entry_seen=false
vector_handled_seen=false
vector_return_seen=false
vector_default_seen=false
svc_handler_seen=false

grep -q 'NovaOS bootstrap kernel svc begin' "${LOG_FILE}" && kernel_begin_seen=true
grep -q 'NovaOS bootstrap kernel svc passed' "${LOG_FILE}" && kernel_success_seen=true
grep -q 'NovaOS bootstrap kernel svc failed' "${LOG_FILE}" && kernel_failure_seen=true
grep -q 'NovaOS bootstrap kernel svc caller capture matched' "${LOG_FILE}" && caller_capture_matched=true
grep -q 'NovaOS bootstrap kernel svc caller capture mismatch' "${LOG_FILE}" && caller_capture_mismatch=true
grep -q '\[VP\]' "${LOG_FILE}" && vector_prestack_seen=true
grep -q 'NovaOS bootstrap vector entered' "${LOG_FILE}" && vector_entry_seen=true
grep -q 'NovaOS bootstrap vector handled' "${LOG_FILE}" && vector_handled_seen=true
grep -q 'NovaOS bootstrap vector return' "${LOG_FILE}" && vector_return_seen=true
grep -q 'NovaOS bootstrap vector default' "${LOG_FILE}" && vector_default_seen=true
grep -q 'bootstrap live svc from initd' "${LOG_FILE}" && svc_handler_seen=true

overall_status="kernel_probe_not_reached"
exit_code=1
if ${kernel_success_seen}; then
  overall_status="returned"
  exit_code=0
elif ${svc_handler_seen}; then
  overall_status="handler_seen_no_return"
  exit_code=0
elif ${vector_return_seen}; then
  overall_status="vector_return_no_probe_return"
  exit_code=0
elif ${vector_default_seen}; then
  overall_status="vector_default"
  exit_code=0
elif ${vector_handled_seen}; then
  overall_status="vector_handled_no_handler_log"
  exit_code=0
elif ${vector_entry_seen}; then
  overall_status="vector_entry_no_handler"
  exit_code=0
elif ${vector_prestack_seen}; then
  overall_status="vector_prestack_no_postsave"
  exit_code=0
elif ${kernel_failure_seen}; then
  overall_status="probe_failed_no_exception"
  exit_code=0
elif ${kernel_begin_seen}; then
  overall_status="reached_kernel_no_vector"
  exit_code=0
fi

novaos_refresh_latest_link "${LOG_FILE}" "${LATEST_LOG_LINK}"
novaos_write_latest_path "${LOG_FILE}" "${LATEST_LOG_PATH}"
novaos_write_latest_status "${LATEST_STATUS_FILE}" \
  "generated_at_utc=${STAMP}" \
  "log_file=${LOG_FILE}" \
  "runner_status=${runner_status}" \
  "overall_status=${overall_status}" \
  "kernel_begin_seen=${kernel_begin_seen}" \
  "kernel_success_seen=${kernel_success_seen}" \
  "kernel_failure_seen=${kernel_failure_seen}" \
  "caller_capture_matched=${caller_capture_matched}" \
  "caller_capture_mismatch=${caller_capture_mismatch}" \
  "vector_prestack_seen=${vector_prestack_seen}" \
  "vector_entry_seen=${vector_entry_seen}" \
  "vector_handled_seen=${vector_handled_seen}" \
  "vector_return_seen=${vector_return_seen}" \
  "vector_default_seen=${vector_default_seen}" \
  "svc_handler_seen=${svc_handler_seen}" \
  "latest_log_link=${LATEST_LOG_LINK}"

exit "${exit_code}"
