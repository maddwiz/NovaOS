#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${ROOT_DIR:-/home/nova/NovaOS}"
REPORT_DIR="${REPORT_DIR:-${ROOT_DIR}/artifacts/reports}"
TIMEOUT_SECONDS="${TIMEOUT_SECONDS:-12}"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
LOG_FILE="${REPORT_DIR}/bootstrap-el0-diagnostic-${STAMP}.log"
LATEST_LOG_LINK="${REPORT_DIR}/latest-bootstrap-el0-diagnostic.log"
LATEST_LOG_PATH="${REPORT_DIR}/latest-bootstrap-el0-diagnostic-path.txt"
LATEST_STATUS_FILE="${REPORT_DIR}/latest-bootstrap-el0-diagnostic-status.txt"

source "${ROOT_DIR}/scripts/novaos-latest.sh"

mkdir -p "${REPORT_DIR}"

set +e
EFI_BINARY=novaaa64 \
EXPECT_TEXTS="NovaOS stage0 loader;NovaOS stage1 entered;NovaOS stage1 bootinfo_v2 sidecar;NovaOS kernel entered;NovaOS kernel bring-up;init capsule summary observed;bootstrap task current initd;bootstrap task launch plan from bootinfo_v2;bootstrap el0 mapping ready;bootstrap task boundary same-el;bootstrap task target boundary drop-to-el0;bootstrap task transfer initd;bootstrap lower-el svc from initd;NovaOS bootstrap vector return" \
STAGE_NOVA_PAYLOADS=1 \
PAYLOAD_FEATURES=qemu_virt_trace,bootstrap_trap_vector_trace,bootstrap_el0_probe \
INITD_FEATURES=qemu_virt_trace,bootstrap_svc_probe,bootstrap_el0_probe \
NOVAAA64_FEATURES=qemu_virt_trace \
TIMEOUT_SECONDS="${TIMEOUT_SECONDS}" \
  "${ROOT_DIR}/scripts/run-qemu-spark-observe.sh" > "${LOG_FILE}" 2>&1
runner_status=$?
set -e

head -c 24000 "${LOG_FILE}"
if [ "$(wc -c < "${LOG_FILE}")" -gt 40000 ]; then
  printf '\n%s\n' '... truncated diagnostic log; showing tail ...'
  tail -c 16000 "${LOG_FILE}"
fi

after_transfer_contains() {
  local pattern="$1"
  awk -v pattern="${pattern}" '
    seen && index($0, pattern) { found = 1 }
    index($0, "bootstrap task target boundary current_el") { seen = 1 }
    END { exit found ? 0 : 1 }
  ' "${LOG_FILE}"
}

payload_entered_seen=false
context_ready_seen=false
kernel_call_seen=false
svc_begin_seen=false
current_el_handler_seen=false
lower_el_handler_seen=false
svc_return_seen=false
instruction_abort_seen=false
data_abort_seen=false
vector_entry_seen=false
vector_handled_seen=false
vector_return_seen=false
vector_default_seen=false

after_transfer_contains 'NovaOS initd payload entered' && payload_entered_seen=true
after_transfer_contains 'NovaOS initd bootstrap context ready' && context_ready_seen=true
after_transfer_contains 'NovaOS initd bootstrap kernel call passed' && kernel_call_seen=true
after_transfer_contains 'NovaOS initd bootstrap svc begin' && svc_begin_seen=true
after_transfer_contains 'bootstrap live svc from initd' && current_el_handler_seen=true
after_transfer_contains 'bootstrap lower-el svc from initd' && lower_el_handler_seen=true
after_transfer_contains 'NovaOS initd bootstrap svc passed' && svc_return_seen=true
after_transfer_contains 'instruction_abort_lower_el' && instruction_abort_seen=true
after_transfer_contains 'data_abort_lower_el' && data_abort_seen=true
after_transfer_contains 'NovaOS bootstrap vector entered' && vector_entry_seen=true
after_transfer_contains 'NovaOS bootstrap vector handled' && vector_handled_seen=true
after_transfer_contains 'NovaOS bootstrap vector return' && vector_return_seen=true
after_transfer_contains 'NovaOS bootstrap vector default' && vector_default_seen=true

overall_status="el0_not_reached"
exit_code=1
if ${lower_el_handler_seen} && ${svc_return_seen}; then
  overall_status="returned"
  exit_code=0
elif ${lower_el_handler_seen} && ${vector_return_seen}; then
  overall_status="svc_returned_to_el0_spin"
  exit_code=0
elif ${lower_el_handler_seen}; then
  overall_status="lower_el_handler_seen_no_return"
  exit_code=0
elif ${data_abort_seen}; then
  overall_status="data_abort_lower_el"
  exit_code=0
elif ${instruction_abort_seen}; then
  overall_status="instruction_abort_lower_el"
  exit_code=0
elif ${current_el_handler_seen}; then
  overall_status="current_el_handler_regression"
  exit_code=0
elif ${vector_default_seen}; then
  overall_status="vector_default"
  exit_code=0
elif ${vector_handled_seen}; then
  overall_status="vector_handled_no_return"
  exit_code=0
elif ${vector_entry_seen}; then
  overall_status="vector_entry_no_handler"
  exit_code=0
elif ${svc_begin_seen}; then
  overall_status="svc_reached_no_handler"
  exit_code=0
elif ${payload_entered_seen}; then
  overall_status="payload_entered_no_svc"
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
  "context_ready_seen=${context_ready_seen}" \
  "kernel_call_seen=${kernel_call_seen}" \
  "svc_begin_seen=${svc_begin_seen}" \
  "current_el_handler_seen=${current_el_handler_seen}" \
  "lower_el_handler_seen=${lower_el_handler_seen}" \
  "svc_return_seen=${svc_return_seen}" \
  "instruction_abort_seen=${instruction_abort_seen}" \
  "data_abort_seen=${data_abort_seen}" \
  "vector_entry_seen=${vector_entry_seen}" \
  "vector_handled_seen=${vector_handled_seen}" \
  "vector_return_seen=${vector_return_seen}" \
  "vector_default_seen=${vector_default_seen}" \
  "latest_log_link=${LATEST_LOG_LINK}"

exit "${exit_code}"
