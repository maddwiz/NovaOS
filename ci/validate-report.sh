#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${ROOT_DIR:-/home/nova/NovaOS}"
REPORT_DIR="${REPORT_DIR:-${ROOT_DIR}/artifacts/reports}"

latest_report="$(ls -1t "${REPORT_DIR}"/novaos-report-*.md 2>/dev/null | head -n 1 || true)"

if [ -z "${latest_report}" ]; then
  printf 'no_report_found\n' >&2
  exit 1
fi

printf '%s\n' "${latest_report}"
