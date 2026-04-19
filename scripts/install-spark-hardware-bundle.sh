#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${ROOT_DIR:-/home/nova/NovaOS}"
ARTIFACT_DIR="${ARTIFACT_DIR:-${ROOT_DIR}/artifacts/hardware}"
MODE="${MODE:-${1:-}}"
ESP_MOUNT="${ESP_MOUNT:-/boot/efi}"
BUNDLE_DIR="${BUNDLE_DIR:-}"
USE_BOOTNEXT="${USE_BOOTNEXT:-0}"
REBOOT_AFTER="${REBOOT_AFTER:-0}"
BACKUP_EXISTING="${BACKUP_EXISTING:-1}"
PURGE_STALE_NOVA="${PURGE_STALE_NOVA:-0}"
BOOT_LABEL_PREFIX="${BOOT_LABEL_PREFIX:-NovaOS}"
export PATH="/home/linuxbrew/.linuxbrew/bin:/home/nova/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:${PATH:-}"
source "${ROOT_DIR}/scripts/novaos-latest.sh"

read_status_value() {
  local file_path="$1"
  local key="$2"
  local value

  if [ ! -f "${file_path}" ]; then
    return 1
  fi

  value="$(grep -E "^${key}=" "${file_path}" | tail -n 1 | cut -d= -f2- || true)"
  if [ -z "${value}" ]; then
    return 1
  fi

  printf '%s\n' "${value}"
}

resolve_bundle_kind() {
  local value="$1"

  case "${value}" in
    ""|spark-observe|observatory)
      printf 'observatory\n'
      ;;
    novaaa64|loader)
      printf 'loader\n'
      ;;
    *)
      return 1
      ;;
  esac
}

set_bundle_selection() {
  case "$1" in
    observatory)
      bundle_prefix="spark-observe"
      mode_name="spark-observe"
      efi_binary="spark-observe"
      ;;
    loader)
      bundle_prefix="novaaa64"
      mode_name="novaaa64"
      efi_binary="novaaa64"
      ;;
    *)
      printf 'unsupported_bundle_kind=%s\n' "$1" >&2
      exit 1
      ;;
  esac
}

infer_bundle_kind_from_dir() {
  local dir_path="$1"

  if [ -d "${dir_path}/nova" ] || [ -f "${dir_path}/novaaa64.efi" ]; then
    printf 'loader\n'
    return 0
  fi

  if [ -f "${dir_path}/spark-observe.efi" ]; then
    printf 'observatory\n'
    return 0
  fi

  return 1
}

schedule_boot_next() {
  local boot_label="$1"
  local loader_path="$2"
  local esp_source esp_disk esp_part bootnum

  if ! command -v efibootmgr >/dev/null 2>&1; then
    printf 'missing_efibootmgr\n' >&2
    exit 1
  fi

  if [ ! -d /sys/firmware/efi ]; then
    printf 'missing_efi_runtime\n' >&2
    exit 1
  fi

  esp_source="$(findmnt -n -o SOURCE --target "${ESP_MOUNT}" || true)"
  if [[ -z "${esp_source}" || "${esp_source}" != /dev/* ]]; then
    printf 'unsupported_esp_source=%s\n' "${esp_source:-unset}" >&2
    exit 1
  fi

  read -r esp_disk esp_part <<EOF
$(lsblk -npo PKNAME,PARTN "${esp_source}" | awk 'NR==1 {print $1, $2}')
EOF

  if [ -z "${esp_disk}" ] || [ -z "${esp_part}" ]; then
    printf 'missing_esp_disk_metadata=%s\n' "${esp_source}" >&2
    exit 1
  fi

  efibootmgr -c -d "${esp_disk}" -p "${esp_part}" -L "${boot_label}" -l "${loader_path}" >/dev/null
  bootnum="$(efibootmgr | sed -n "s/^Boot\\([0-9A-Fa-f]\\{4\\}\\)\\* ${boot_label}$/\\1/p" | tail -n 1)"
  if [ -z "${bootnum}" ]; then
    printf 'missing_boot_entry_for_label=%s\n' "${boot_label}" >&2
    exit 1
  fi

  efibootmgr -n "${bootnum}" >/dev/null
  printf '%s\n' "${bootnum}"
}

if [ -z "${BUNDLE_DIR}" ] && [ -d "${MODE}" ]; then
  BUNDLE_DIR="${MODE}"
  MODE=""
fi

if [ -n "${BUNDLE_DIR}" ] && [ -z "${MODE}" ]; then
  MODE="$(infer_bundle_kind_from_dir "${BUNDLE_DIR}" || true)"
fi

BUNDLE_KIND="$(resolve_bundle_kind "${MODE}")"
set_bundle_selection "${BUNDLE_KIND}"

if [ -z "${BUNDLE_DIR}" ]; then
  latest_bundle_path_file="${ARTIFACT_DIR}/latest-${bundle_prefix}-bundle-path.txt"
  if [ ! -f "${latest_bundle_path_file}" ]; then
    printf 'missing_bundle_path_file=%s\n' "${latest_bundle_path_file}" >&2
    exit 1
  fi
  BUNDLE_DIR="$(tail -n 1 "${latest_bundle_path_file}")"
fi

if [ ! -d "${BUNDLE_DIR}" ]; then
  printf 'missing_bundle_dir=%s\n' "${BUNDLE_DIR}" >&2
  exit 1
fi

bundle_manifest="${BUNDLE_DIR}/bundle-manifest.txt"
if [ -f "${bundle_manifest}" ]; then
  manifest_kind="$(read_status_value "${bundle_manifest}" "bundle_kind" || true)"
  manifest_mode="$(read_status_value "${bundle_manifest}" "mode" || true)"
  manifest_binary="$(read_status_value "${bundle_manifest}" "efi_binary" || true)"

  if [ -n "${manifest_kind}" ]; then
    BUNDLE_KIND="${manifest_kind}"
    set_bundle_selection "${BUNDLE_KIND}"
  fi
  if [ -n "${manifest_mode}" ]; then
    mode_name="${manifest_mode}"
  fi
  if [ -n "${manifest_binary}" ]; then
    efi_binary="${manifest_binary}"
  fi
fi

bundle_bootaa64="${BUNDLE_DIR}/EFI/BOOT/BOOTAA64.EFI"
bundle_mode_efi="${BUNDLE_DIR}/${efi_binary}.efi"
bundle_readme="${BUNDLE_DIR}/README.txt"
bundle_has_nova=0
report_target=""
if [ -d "${BUNDLE_DIR}/nova" ]; then
  bundle_has_nova=1
fi

if [ ! -f "${bundle_bootaa64}" ]; then
  printf 'missing_bootaa64_efi=%s\n' "${bundle_bootaa64}" >&2
  exit 1
fi

if [ ! -f "${bundle_mode_efi}" ]; then
  printf 'missing_mode_efi=%s\n' "${bundle_mode_efi}" >&2
  exit 1
fi

mkdir -p "${ESP_MOUNT}"

boot_target="${ESP_MOUNT}/EFI/BOOT/BOOTAA64.EFI"
novaos_dir="${ESP_MOUNT}/EFI/NovaOS"
mode_target="${novaos_dir}/${efi_binary}.efi"
manifest_target="${novaos_dir}/${efi_binary}-bundle-manifest.txt"
readme_target="${novaos_dir}/${efi_binary}-README.txt"
nova_target="${ESP_MOUNT}/nova"
report_target="${nova_target}/observatory"
if [ "${BUNDLE_KIND}" = "loader" ]; then
  report_target="${nova_target}/loader"
fi

stamp="$(date -u +%Y%m%dT%H%M%SZ)"
install_dir="${ARTIFACT_DIR}/installs"
backup_dir="${ARTIFACT_DIR}/install-backups/${bundle_prefix}-${stamp}"
install_manifest="${install_dir}/${bundle_prefix}-install-${stamp}.txt"
latest_install_link="${install_dir}/latest-${bundle_prefix}-install.txt"
latest_install_path="${install_dir}/latest-${bundle_prefix}-install-path.txt"
latest_install_status="${install_dir}/latest-${bundle_prefix}-install-status.txt"

if [ "${BACKUP_EXISTING}" = "1" ]; then
  if [ -f "${boot_target}" ]; then
    mkdir -p "${backup_dir}/EFI/BOOT"
    cp -a "${boot_target}" "${backup_dir}/EFI/BOOT/BOOTAA64.EFI"
  fi
  if [ -f "${mode_target}" ]; then
    mkdir -p "${backup_dir}/EFI/NovaOS"
    cp -a "${mode_target}" "${backup_dir}/EFI/NovaOS/${efi_binary}.efi"
  fi
  if [ -f "${manifest_target}" ]; then
    mkdir -p "${backup_dir}/EFI/NovaOS"
    cp -a "${manifest_target}" "${backup_dir}/EFI/NovaOS/${efi_binary}-bundle-manifest.txt"
  fi
  if [ -f "${readme_target}" ]; then
    mkdir -p "${backup_dir}/EFI/NovaOS"
    cp -a "${readme_target}" "${backup_dir}/EFI/NovaOS/${efi_binary}-README.txt"
  fi
  if [ "${bundle_has_nova}" = "1" ] && [ -d "${nova_target}" ]; then
    mkdir -p "${backup_dir}"
    cp -a "${nova_target}" "${backup_dir}/nova"
  fi
fi

mkdir -p "${ESP_MOUNT}/EFI/BOOT" "${novaos_dir}"
cp -a "${bundle_bootaa64}" "${boot_target}"
cp -a "${bundle_mode_efi}" "${mode_target}"
cp -a "${bundle_manifest}" "${manifest_target}"
if [ -f "${bundle_readme}" ]; then
  cp -a "${bundle_readme}" "${readme_target}"
fi

if [ "${bundle_has_nova}" = "1" ]; then
  rm -rf "${nova_target}"
  cp -a "${BUNDLE_DIR}/nova" "${nova_target}"
elif [ "${PURGE_STALE_NOVA}" = "1" ] && [ -d "${nova_target}" ]; then
  rm -rf "${nova_target}"
fi

mkdir -p "${report_target}"

boot_label="${BOOT_LABEL_PREFIX} ${mode_name}"
boot_entry=""
if [ "${USE_BOOTNEXT}" = "1" ]; then
  boot_entry="$(schedule_boot_next "${boot_label}" "\\\\EFI\\\\NovaOS\\\\${efi_binary}.efi")"
fi

mkdir -p "${install_dir}"
cat > "${install_manifest}" <<EOF
generated_at_utc=${stamp}
mode=${mode_name}
bundle_kind=${BUNDLE_KIND}
bundle_dir=${BUNDLE_DIR}
esp_mount=${ESP_MOUNT}
bootaa64_efi=${boot_target}
mode_efi=${mode_target}
bundle_manifest_target=${manifest_target}
readme_target=${readme_target}
bundle_has_nova=${bundle_has_nova}
report_target=${report_target}
purge_stale_nova=${PURGE_STALE_NOVA}
backup_existing=${BACKUP_EXISTING}
backup_dir=${backup_dir}
bootnext_requested=${USE_BOOTNEXT}
bootnext_entry=${boot_entry}
boot_label=${boot_label}
EOF

novaos_refresh_latest_link "${install_manifest}" "${latest_install_link}"
novaos_write_latest_path "${install_manifest}" "${latest_install_path}"
novaos_write_latest_status "${latest_install_status}" \
  "generated_at_utc=${stamp}" \
  "mode=${mode_name}" \
  "bundle_kind=${BUNDLE_KIND}" \
  "bundle_dir=${BUNDLE_DIR}" \
  "esp_mount=${ESP_MOUNT}" \
  "bootaa64_efi=${boot_target}" \
  "mode_efi=${mode_target}" \
  "bundle_manifest_target=${manifest_target}" \
  "readme_target=${readme_target}" \
  "bundle_has_nova=${bundle_has_nova}" \
  "report_target=${report_target}" \
  "purge_stale_nova=${PURGE_STALE_NOVA}" \
  "backup_existing=${BACKUP_EXISTING}" \
  "backup_dir=${backup_dir}" \
  "bootnext_requested=${USE_BOOTNEXT}" \
  "bootnext_entry=${boot_entry}" \
  "boot_label=${boot_label}" \
  "report_file=${install_manifest}" \
  "latest_report_link=${latest_install_link}"

if [ "${REBOOT_AFTER}" = "1" ]; then
  if [ "${USE_BOOTNEXT}" != "1" ]; then
    printf 'reboot_after_requires_use_bootnext\n' >&2
    exit 1
  fi
  if command -v systemctl >/dev/null 2>&1; then
    systemctl reboot
  else
    reboot
  fi
fi

printf 'mode=%s\n' "${mode_name}"
printf 'bundle_dir=%s\n' "${BUNDLE_DIR}"
printf 'esp_mount=%s\n' "${ESP_MOUNT}"
printf 'bootaa64_efi=%s\n' "${boot_target}"
printf 'mode_efi=%s\n' "${mode_target}"
printf 'bundle_has_nova=%s\n' "${bundle_has_nova}"
printf 'report_target=%s\n' "${report_target}"
printf 'bootnext_requested=%s\n' "${USE_BOOTNEXT}"
printf 'bootnext_entry=%s\n' "${boot_entry}"
printf 'install_manifest=%s\n' "${install_manifest}"
