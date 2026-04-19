#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${ROOT_DIR:-/home/nova/NovaOS}"
USER_UNIT_DIR="${HOME}/.config/systemd/user"
UNIT_NAME="novaos-validation.service"
export PATH="/home/linuxbrew/.linuxbrew/bin:/home/nova/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:${PATH:-}"

mkdir -p "${USER_UNIT_DIR}"
install -m 0644 "${ROOT_DIR}/ci/systemd/${UNIT_NAME}" "${USER_UNIT_DIR}/${UNIT_NAME}"

systemctl --user daemon-reload
systemctl --user enable --now "${UNIT_NAME}"
systemctl --user --no-pager --full status "${UNIT_NAME}" || true
