#!/usr/bin/env bash
set -euo pipefail

novaos_refresh_latest_link() {
  local target_path="$1"
  local latest_link="$2"
  local latest_tmp

  mkdir -p "$(dirname "${latest_link}")"
  latest_tmp="${latest_link}.tmp.$$"
  rm -f "${latest_tmp}"
  ln -s "$(basename "${target_path}")" "${latest_tmp}"
  mv -Tf "${latest_tmp}" "${latest_link}"
}

novaos_write_latest_path() {
  local target_path="$1"
  local latest_path_file="$2"
  local latest_tmp

  mkdir -p "$(dirname "${latest_path_file}")"
  latest_tmp="${latest_path_file}.tmp.$$"
  rm -f "${latest_tmp}"
  printf '%s\n' "${target_path}" > "${latest_tmp}"
  mv -Tf "${latest_tmp}" "${latest_path_file}"
}

novaos_write_latest_status() {
  local latest_status_file="$1"
  shift
  local latest_tmp

  mkdir -p "$(dirname "${latest_status_file}")"
  latest_tmp="${latest_status_file}.tmp.$$"
  rm -f "${latest_tmp}"
  {
    for line in "$@"; do
      printf '%s\n' "${line}"
    done
  } > "${latest_tmp}"
  mv -Tf "${latest_tmp}" "${latest_status_file}"
}
