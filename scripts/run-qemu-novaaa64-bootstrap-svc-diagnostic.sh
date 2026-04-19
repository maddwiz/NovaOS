#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${ROOT_DIR:-/home/nova/NovaOS}"
REPORT_DIR="${REPORT_DIR:-${ROOT_DIR}/artifacts/reports}"
TIMEOUT_SECONDS="${TIMEOUT_SECONDS:-12}"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
LOG_FILE="${REPORT_DIR}/bootstrap-svc-diagnostic-${STAMP}.log"
LATEST_LOG_LINK="${REPORT_DIR}/latest-bootstrap-svc-diagnostic.log"
LATEST_LOG_PATH="${REPORT_DIR}/latest-bootstrap-svc-diagnostic-path.txt"
LATEST_STATUS_FILE="${REPORT_DIR}/latest-bootstrap-svc-diagnostic-status.txt"

source "${ROOT_DIR}/scripts/novaos-latest.sh"

mkdir -p "${REPORT_DIR}"

set +e
EFI_BINARY=novaaa64 \
EXPECT_TEXTS="NovaOS stage0 loader;NovaOS stage1 entered;NovaOS stage1 bootinfo_v2 sidecar;NovaOS kernel entered;NovaOS kernel bring-up;init capsule summary observed;bootstrap task current initd;bootstrap task launch plan from bootinfo_v2;bootstrap capability probe passed;bootstrap endpoint probe passed;bootstrap task transfer initd;NovaOS initd payload entered;NovaOS initd bootstrap context ready;bootstrap kernel call from initd;NovaOS initd bootstrap kernel call passed;NovaOS initd bootstrap svc begin" \
STAGE_NOVA_PAYLOADS=1 \
PAYLOAD_FEATURES=qemu_virt_trace,bootstrap_trap_vector_trace \
INITD_FEATURES=qemu_virt_trace,bootstrap_svc_probe \
NOVAAA64_FEATURES=qemu_virt_trace \
TIMEOUT_SECONDS="${TIMEOUT_SECONDS}" \
  "${ROOT_DIR}/scripts/run-qemu-spark-observe.sh" > "${LOG_FILE}" 2>&1
runner_status=$?
set -e

cat "${LOG_FILE}"

payload_entered_seen=false
kernel_call_seen=false
svc_begin_seen=false
vector_prestack_seen=false
vector_entry_seen=false
vector_handled_seen=false
vector_return_seen=false
vector_default_seen=false
svc_handler_seen=false
svc_return_seen=false

grep -q 'NovaOS initd payload entered' "${LOG_FILE}" && payload_entered_seen=true
grep -q 'NovaOS initd bootstrap kernel call passed' "${LOG_FILE}" && kernel_call_seen=true
grep -q 'NovaOS initd bootstrap svc begin' "${LOG_FILE}" && svc_begin_seen=true
grep -q '\[VP\]' "${LOG_FILE}" && vector_prestack_seen=true
grep -q 'NovaOS bootstrap vector entered' "${LOG_FILE}" && vector_entry_seen=true
grep -q 'NovaOS bootstrap vector handled' "${LOG_FILE}" && vector_handled_seen=true
grep -q 'NovaOS bootstrap vector return' "${LOG_FILE}" && vector_return_seen=true
grep -q 'NovaOS bootstrap vector default' "${LOG_FILE}" && vector_default_seen=true
grep -q 'bootstrap live svc from initd' "${LOG_FILE}" && svc_handler_seen=true
grep -q 'NovaOS initd bootstrap svc passed' "${LOG_FILE}" && svc_return_seen=true

overall_status="svc_not_reached"
exit_code=1
if ${svc_return_seen}; then
  overall_status="returned"
  exit_code=0
elif ${svc_handler_seen}; then
  overall_status="handler_seen_no_return"
  exit_code=0
elif ${vector_return_seen}; then
  overall_status="vector_return_no_payload_return"
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
elif ${svc_begin_seen}; then
  overall_status="reached_svc_no_vector"
  exit_code=0
fi

novaos_refresh_latest_link "${LOG_FILE}" "${LATEST_LOG_LINK}"
novaos_write_latest_path "${LOG_FILE}" "${LATEST_LOG_PATH}"
novaos_write_latest_status "${LATEST_STATUS_FILE}" \
  "generated_at_utc=${STAMP}" \
  "log_file=${LOG_FILE}" \
  "runner_status=${runner_status}" \
  "overall_status=${overall_status}" \
  "payload_entered_seen=${payload_entered_seen}" \
  "kernel_call_seen=${kernel_call_seen}" \
  "svc_begin_seen=${svc_begin_seen}" \
  "vector_prestack_seen=${vector_prestack_seen}" \
  "vector_entry_seen=${vector_entry_seen}" \
  "vector_handled_seen=${vector_handled_seen}" \
  "vector_return_seen=${vector_return_seen}" \
  "vector_default_seen=${vector_default_seen}" \
  "svc_handler_seen=${svc_handler_seen}" \
  "svc_return_seen=${svc_return_seen}" \
  "latest_log_link=${LATEST_LOG_LINK}"

exit "${exit_code}"
