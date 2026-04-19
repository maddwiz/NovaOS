#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${ROOT_DIR:-/home/nova/NovaOS}"
REPORT_DIR="${REPORT_DIR:-${ROOT_DIR}/artifacts/reports}"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
REPORT_FILE="${REPORT_DIR}/novaos-report-${STAMP}.md"
ENV_CAPTURE="${REPORT_DIR}/novaos-env-${STAMP}.txt"
VALIDATE_CAPTURE="${REPORT_DIR}/novaos-validate-${STAMP}.txt"
LATEST_REPORT_LINK="${REPORT_DIR}/latest-report.md"
LATEST_ENV_LINK="${REPORT_DIR}/latest-env.txt"
LATEST_VALIDATE_LINK="${REPORT_DIR}/latest-validate.txt"
LATEST_REPORT_PATH="${REPORT_DIR}/latest-report-path.txt"
LATEST_STATUS_FILE="${REPORT_DIR}/latest-status.txt"

source "${ROOT_DIR}/scripts/novaos-latest.sh"

mkdir -p "${REPORT_DIR}"

env_status=0
validate_status=0

set +e
"${ROOT_DIR}/scripts/novaos-env-check.sh" > "${ENV_CAPTURE}" 2>&1
env_status=$?
"${ROOT_DIR}/ci/validate-local.sh" > "${VALIDATE_CAPTURE}" 2>&1
validate_status=$?
set -e

{
  printf '%s\n' '# NovaOS Validation Report'
  printf '\n'
  printf '%s\n' "- generated_at_utc: ${STAMP}"
  printf '%s\n' "- root: ${ROOT_DIR}"
  printf '%s\n' "- host: $(uname -a)"
  printf '%s\n' "- environment_status: ${env_status}"
  printf '%s\n' "- local_validation_status: ${validate_status}"
  printf '\n## Environment\n\n'
  printf '```\n'
  cat "${ENV_CAPTURE}"
  printf '```\n'
  printf '\n## Local Validation\n\n'
  printf '```\n'
  cat "${VALIDATE_CAPTURE}"
  printf '```\n'
} > "${REPORT_FILE}"

novaos_refresh_latest_link "${REPORT_FILE}" "${LATEST_REPORT_LINK}"
novaos_refresh_latest_link "${ENV_CAPTURE}" "${LATEST_ENV_LINK}"
novaos_refresh_latest_link "${VALIDATE_CAPTURE}" "${LATEST_VALIDATE_LINK}"
novaos_write_latest_path "${REPORT_FILE}" "${LATEST_REPORT_PATH}"
novaos_write_latest_status "${LATEST_STATUS_FILE}" \
  "generated_at_utc=${STAMP}" \
  "report_file=${REPORT_FILE}" \
  "environment_status=${env_status}" \
  "local_validation_status=${validate_status}" \
  "latest_report_link=${LATEST_REPORT_LINK}" \
  "latest_env_link=${LATEST_ENV_LINK}" \
  "latest_validate_link=${LATEST_VALIDATE_LINK}"

bash "${ROOT_DIR}/scripts/update-roadmap-status.sh"

printf '%s\n' "${REPORT_FILE}"
