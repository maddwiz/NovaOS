#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${ROOT_DIR:-$(cd -- "${SCRIPT_DIR}/.." && pwd)}"
USER_UNIT_DIR="${HOME}/.config/systemd/user"
UNIT_NAME="novaos-validation.service"
UNIT_TEMPLATE="${ROOT_DIR}/ci/systemd/${UNIT_NAME}.in"
export PATH="/home/linuxbrew/.linuxbrew/bin:/home/nova/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:${PATH:-}"

mkdir -p "${USER_UNIT_DIR}"
unit_tmp="$(mktemp)"
trap 'rm -f "${unit_tmp}"' EXIT
awk -v root="${ROOT_DIR}" '{ gsub("__NOVAOS_ROOT_DIR__", root); print }' \
  "${UNIT_TEMPLATE}" > "${unit_tmp}"
install -m 0644 "${unit_tmp}" "${USER_UNIT_DIR}/${UNIT_NAME}"

systemctl --user daemon-reload
systemctl --user enable --now "${UNIT_NAME}"
systemctl --user --no-pager --full status "${UNIT_NAME}" || true
