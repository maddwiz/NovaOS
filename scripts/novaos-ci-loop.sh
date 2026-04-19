#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${ROOT_DIR:-/home/nova/NovaOS}"
ITERATIONS="${ITERATIONS:-0}"
SLEEP_SECONDS="${SLEEP_SECONDS:-300}"
RUN_ONCE="${RUN_ONCE:-0}"

source "${ROOT_DIR}/scripts/novaos-latest.sh"

iteration=1
while :; do
  stamp="$(date -u +%Y%m%dT%H%M%SZ)"
  report_dir="${ROOT_DIR}/artifacts/reports"
  mkdir -p "${report_dir}"

  log_file="${report_dir}/novaos-loop-${stamp}.log"
  summary_file="${report_dir}/novaos-loop-${stamp}.summary"
  latest_log_link="${report_dir}/latest-loop.log"
  latest_summary_link="${report_dir}/latest-loop.summary"
  latest_loop_path="${report_dir}/latest-loop-path.txt"
  latest_loop_status="${report_dir}/latest-loop-status.txt"

  {
    printf 'NovaOS continuous validation loop\n'
    printf 'mode=continuous_build_and_validation\n'
    printf 'timestamp_utc=%s\n' "${stamp}"
    printf 'iteration=%s\n' "${iteration}"
  } | tee "${summary_file}" >/dev/null

  env_status=0
  validate_status=0
  report_status=0
  set +e
  {
    printf '== env ==\n'
    "${ROOT_DIR}/scripts/novaos-env-check.sh"
  } | tee "${log_file}"
  env_status=$?

  {
    printf '\n== validate ==\n'
    "${ROOT_DIR}/ci/validate-local.sh"
  } | tee -a "${log_file}"
  validate_status=$?

  {
    printf '\n== report ==\n'
    "${ROOT_DIR}/scripts/novaos-report.sh"
  } | tee -a "${log_file}"
  report_status=$?
  set -e

  {
    printf 'env_status=%s\n' "${env_status}"
    printf 'validate_status=%s\n' "${validate_status}"
    printf 'report_status=%s\n' "${report_status}"
  } >> "${summary_file}"

  novaos_refresh_latest_link "${log_file}" "${latest_log_link}"
  novaos_refresh_latest_link "${summary_file}" "${latest_summary_link}"
  novaos_write_latest_path "${summary_file}" "${latest_loop_path}"
  novaos_write_latest_status "${latest_loop_status}" \
    "timestamp_utc=${stamp}" \
    "iteration=${iteration}" \
    "log_file=${log_file}" \
    "summary_file=${summary_file}" \
    "env_status=${env_status}" \
    "validate_status=${validate_status}" \
    "report_status=${report_status}" \
    "latest_log_link=${latest_log_link}" \
    "latest_summary_link=${latest_summary_link}"

  bash "${ROOT_DIR}/scripts/update-roadmap-status.sh"

  if [ "${env_status}" -ne 0 ] || [ "${validate_status}" -ne 0 ] || [ "${report_status}" -ne 0 ]; then
    printf 'NovaOS build/validation iteration failed; continuing to the next loop.\n' >&2
  fi

  if [ "${RUN_ONCE}" = "1" ]; then
    break
  fi

  if [ "${ITERATIONS}" != "0" ] && [ "${iteration}" -ge "${ITERATIONS}" ]; then
    break
  fi

  iteration=$((iteration + 1))
  sleep "${SLEEP_SECONDS}"
done

if [ "${env_status}" -ne 0 ] || [ "${validate_status}" -ne 0 ] || [ "${report_status}" -ne 0 ]; then
  exit 1
fi
