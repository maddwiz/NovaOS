#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${ROOT_DIR:-/home/nova/NovaOS}"
export PATH="/home/linuxbrew/.linuxbrew/bin:/home/nova/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:${PATH:-}"

for script in \
  "${ROOT_DIR}/scripts/novaos-latest.sh" \
  "${ROOT_DIR}/scripts/novaos-env-check.sh" \
  "${ROOT_DIR}/scripts/build-initd.sh" \
  "${ROOT_DIR}/scripts/build-init-capsule.sh" \
  "${ROOT_DIR}/scripts/prepare-spark-hardware-bundle.sh" \
  "${ROOT_DIR}/scripts/install-spark-hardware-bundle.sh" \
  "${ROOT_DIR}/scripts/validate-payloads.sh" \
  "${ROOT_DIR}/scripts/compare-spark-observe-reports.sh" \
  "${ROOT_DIR}/scripts/check-novaaa64-loader-report.sh" \
  "${ROOT_DIR}/scripts/check-spark-stage-chain-proof.sh" \
  "${ROOT_DIR}/scripts/collect-spark-observe-report.sh" \
  "${ROOT_DIR}/scripts/collect-novaaa64-loader-report.sh" \
  "${ROOT_DIR}/scripts/complete-spark-hardware-proof.sh" \
  "${ROOT_DIR}/scripts/finalize-spark-hardware-proof.sh" \
  "${ROOT_DIR}/scripts/update-roadmap-status.sh" \
  "${ROOT_DIR}/scripts/novaos-report.sh" \
  "${ROOT_DIR}/scripts/novaos-ci-loop.sh" \
  "${ROOT_DIR}/scripts/run-qemu-novaaa64-bootstrap-kernel-svc-diagnostic.sh" \
  "${ROOT_DIR}/scripts/run-qemu-novaaa64-bootstrap-trap-diagnostic.sh" \
  "${ROOT_DIR}/scripts/run-qemu-novaaa64-bootstrap-svc-diagnostic.sh" \
  "${ROOT_DIR}/scripts/run-qemu-novaaa64-bootstrap-pretransfer-svc-diagnostic.sh" \
  "${ROOT_DIR}/ci/validate-local.sh" \
  "${ROOT_DIR}/ci/validate-report.sh" \
  "${ROOT_DIR}/toolchain/novaos-prereqs.sh"
do
  bash -n "${script}"
done

"${ROOT_DIR}/scripts/novaos-env-check.sh"
cc -std=c11 -fsyntax-only "${ROOT_DIR}/abi/boot/nova_bootinfo_v1.h"
cc -std=c11 -fsyntax-only "${ROOT_DIR}/abi/boot/nova_bootinfo_v2.h"
cc -std=c11 -fsyntax-only "${ROOT_DIR}/abi/boot/nova_image_digest_v1.h"
cc -std=c11 -fsyntax-only "${ROOT_DIR}/abi/boot/nova_verification_v1.h"
cc -std=c11 -fsyntax-only "${ROOT_DIR}/abi/capsule/nova_init_capsule_v1.h"
cc -std=c11 -fsyntax-only "${ROOT_DIR}/abi/payload/nova_payload_v1.h"
cc -std=c11 -fsyntax-only "${ROOT_DIR}/abi/syscall/nova_syscall_v1.h"
cargo metadata --format-version 1 --no-deps >/dev/null
cargo check --workspace
cargo test -p nova_fabric -p nova_rt -p novaos-acceld -p novaos-initd -p novaos-memd -p novaos-stage1 -p novaos-kernel -p novaos-kernel-x86_64 -p novaos-pci
cargo check -p spark-observe --target aarch64-unknown-uefi
cargo check -p novaaa64 --target aarch64-unknown-uefi
cargo check -p novaos-kernel --target aarch64-unknown-none-softfloat
cargo check -p novaos-kernel-x86_64
cargo check -p novaos-stage1
"${ROOT_DIR}/scripts/build-efi.sh" >/dev/null
"${ROOT_DIR}/scripts/build-kernel.sh" >/dev/null
printf 'payload_build=pass\n'
INITD_FEATURES="qemu_virt_trace,bootstrap_trap_probe" \
  "${ROOT_DIR}/scripts/build-initd.sh" >/dev/null
printf 'initd_trap_probe_build=pass\n'
INITD_FEATURES="qemu_virt_trace,bootstrap_svc_probe" \
  "${ROOT_DIR}/scripts/build-initd.sh" >/dev/null
printf 'initd_svc_probe_build=pass\n'
PAYLOAD_FEATURES="qemu_virt_trace,bootstrap_trap_vector_trace" \
  "${ROOT_DIR}/scripts/build-kernel.sh" >/dev/null
printf 'payload_trap_vector_build=pass\n'
PAYLOAD_FEATURES="qemu_virt_trace,bootstrap_trap_vector_trace,bootstrap_kernel_svc_probe" \
  "${ROOT_DIR}/scripts/build-kernel.sh" >/dev/null
printf 'payload_kernel_svc_probe_build=pass\n'
PAYLOAD_FEATURES="qemu_virt_trace,bootstrap_trap_vector_trace,bootstrap_pretransfer_svc_probe" \
  "${ROOT_DIR}/scripts/build-kernel.sh" >/dev/null
printf 'payload_pretransfer_svc_probe_build=pass\n'
"${ROOT_DIR}/scripts/validate-payloads.sh"
esp_test_dir="$(mktemp -d)"
init_capsule_build_path="$(mktemp)"
bundle_test_dir="$(mktemp -d)"
bundle_loader_dir="$(mktemp -d)"
install_artifact_dir="$(mktemp -d)"
collect_report_dir="$(mktemp -d)"
collect_loader_report_dir="$(mktemp -d)"
proof_finalize_dir="$(mktemp -d)"
proof_complete_dir="$(mktemp -d)"
trap 'rm -rf "${esp_test_dir}" "${init_capsule_build_path}" "${bundle_test_dir}" "${bundle_loader_dir}" "${install_artifact_dir}" "${collect_report_dir}" "${collect_loader_report_dir}" "${proof_finalize_dir}" "${proof_complete_dir}"' EXIT
PROFILE=dev bash "${ROOT_DIR}/scripts/build-init-capsule.sh" "${init_capsule_build_path}" >/dev/null
test -s "${init_capsule_build_path}"
printf 'init_capsule_build=pass\n'
mkdir -p "${bundle_test_dir}/EFI/BOOT"
cp "${ROOT_DIR}/target/aarch64-unknown-uefi/debug/spark-observe.efi" \
  "${bundle_test_dir}/EFI/BOOT/BOOTAA64.EFI"
cp "${ROOT_DIR}/target/aarch64-unknown-uefi/debug/spark-observe.efi" \
  "${bundle_test_dir}/spark-observe.efi"
cat > "${bundle_test_dir}/bundle-manifest.txt" <<EOF
bundle_kind=observatory
mode=spark-observe
efi_binary=spark-observe
EOF
ARTIFACT_DIR="${install_artifact_dir}" ESP_MOUNT="${esp_test_dir}" BUNDLE_DIR="${bundle_test_dir}" \
  bash "${ROOT_DIR}/scripts/install-spark-hardware-bundle.sh" spark-observe >/dev/null
test -s "${esp_test_dir}/EFI/BOOT/BOOTAA64.EFI"
test -s "${esp_test_dir}/EFI/NovaOS/spark-observe.efi"
test -d "${esp_test_dir}/nova/observatory"
printf 'spark_observatory_install=pass\n'
mkdir -p "${bundle_loader_dir}/EFI/BOOT" "${bundle_loader_dir}/nova"
cp "${ROOT_DIR}/target/aarch64-unknown-uefi/debug/novaaa64.efi" \
  "${bundle_loader_dir}/EFI/BOOT/BOOTAA64.EFI"
cp "${ROOT_DIR}/target/aarch64-unknown-uefi/debug/novaaa64.efi" \
  "${bundle_loader_dir}/novaaa64.efi"
cp "${ROOT_DIR}/target/aarch64-unknown-none-softfloat/debug/stage1-payload.bin" \
  "${bundle_loader_dir}/nova/stage1.bin"
cp "${ROOT_DIR}/target/aarch64-unknown-none-softfloat/debug/kernel-payload.bin" \
  "${bundle_loader_dir}/nova/kernel.bin"
cp "${init_capsule_build_path}" "${bundle_loader_dir}/nova/init.capsule"
cat > "${bundle_loader_dir}/bundle-manifest.txt" <<EOF
bundle_kind=loader
mode=novaaa64
efi_binary=novaaa64
EOF
ARTIFACT_DIR="${install_artifact_dir}" ESP_MOUNT="${esp_test_dir}" BUNDLE_DIR="${bundle_loader_dir}" \
  bash "${ROOT_DIR}/scripts/install-spark-hardware-bundle.sh" novaaa64 >/dev/null
test -s "${esp_test_dir}/EFI/BOOT/BOOTAA64.EFI"
test -s "${esp_test_dir}/EFI/NovaOS/novaaa64.efi"
test -s "${esp_test_dir}/nova/stage1.bin"
test -s "${esp_test_dir}/nova/kernel.bin"
test -s "${esp_test_dir}/nova/init.capsule"
test -d "${esp_test_dir}/nova/loader"
printf 'spark_loader_install=pass\n'
TIMEOUT_SECONDS=12 "${ROOT_DIR}/scripts/run-qemu-spark-observe.sh" >/dev/null
printf 'qemu_smoke=pass\n'
spark_observe_report="${ROOT_DIR}/artifacts/reports/latest-spark-observe-report.txt"
test -s "${spark_observe_report}"
grep -q '^report_kind=spark_observatory_v2_seed_report$' "${spark_observe_report}"
grep -q '^display_seed_count=' "${spark_observe_report}"
grep -q '^storage_seed_count=' "${spark_observe_report}"
grep -q '^network_seed_count=' "${spark_observe_report}"
grep -q '^accel_seed_draft_count=' "${spark_observe_report}"
printf 'spark_observatory_report=pass\n'
"${ROOT_DIR}/scripts/compare-spark-observe-reports.sh" \
  "${spark_observe_report}" \
  "${spark_observe_report}" \
  >/dev/null
printf 'spark_observatory_compare=pass\n'
REPORT_DIR="${collect_report_dir}" BASELINE_REPORT="${spark_observe_report}" \
  bash "${ROOT_DIR}/scripts/collect-spark-observe-report.sh" "${spark_observe_report}" >/dev/null
test -s "${collect_report_dir}/latest-spark-observe-real-report.txt"
printf 'spark_observatory_collect=pass\n'
TIMEOUT_SECONDS=12 "${ROOT_DIR}/scripts/run-qemu-novaaa64.sh" >/dev/null
printf 'qemu_stage0_smoke=pass\n'
loader_handoff_report="${ROOT_DIR}/artifacts/reports/latest-novaaa64-loader-report.txt"
test -s "${loader_handoff_report}"
grep -q '^report_kind=novaaa64_loader_handoff_report$' "${loader_handoff_report}"
grep -q '^stage1_plan_ready=true$' "${loader_handoff_report}"
grep -q '^boot_info_v2_valid=true$' "${loader_handoff_report}"
printf 'qemu_loader_handoff_report=pass\n'
REPORT_DIR="${collect_loader_report_dir}" BASELINE_REPORT="${loader_handoff_report}" \
  bash "${ROOT_DIR}/scripts/collect-novaaa64-loader-report.sh" "${loader_handoff_report}" >/dev/null
test -s "${collect_loader_report_dir}/latest-novaaa64-loader-real-report.txt"
test -s "${collect_loader_report_dir}/latest-novaaa64-loader-check-status.txt"
printf 'qemu_loader_collect=pass\n'
cat > "${proof_finalize_dir}/qemu-stage-chain-proof.txt" <<'EOF'
NovaOS stage0 loader
NovaOS stage0 post-exit
NovaOS stage1 entered
NovaOS stage1 bootinfo_v2 sidecar
NovaOS kernel entered
EOF
REPORT_DIR="${proof_finalize_dir}" \
  bash "${ROOT_DIR}/scripts/check-spark-stage-chain-proof.sh" \
  "${proof_finalize_dir}/qemu-stage-chain-proof.txt" >/dev/null
test -s "${proof_finalize_dir}/latest-spark-stage-chain-check-status.txt"
grep -q '^status=pass$' "${proof_finalize_dir}/latest-spark-stage-chain-check-status.txt"
printf 'spark_stage_chain_check=pass\n'
REPORT_DIR="${proof_finalize_dir}" \
  OBSERVATORY_REAL_STATUS="${collect_report_dir}/latest-spark-observe-real-status.txt" \
  OBSERVATORY_COMPARE_STATUS="${collect_report_dir}/latest-spark-observe-compare-status.txt" \
  LOADER_REAL_STATUS="${collect_loader_report_dir}/latest-novaaa64-loader-real-status.txt" \
  LOADER_CHECK_STATUS="${collect_loader_report_dir}/latest-novaaa64-loader-check-status.txt" \
  bash "${ROOT_DIR}/scripts/finalize-spark-hardware-proof.sh" \
  "${proof_finalize_dir}/qemu-stage-chain-proof.txt" >/dev/null
test -s "${proof_finalize_dir}/latest-spark-hardware-proof-status.txt"
grep -q '^overall_status=pass$' "${proof_finalize_dir}/latest-spark-hardware-proof-status.txt"
grep -q '^stage_chain_check_status=pass$' "${proof_finalize_dir}/latest-spark-hardware-proof-status.txt"
printf 'spark_hardware_proof_finalize=pass\n'
REPORT_DIR="${proof_complete_dir}" \
OBSERVATORY_SOURCE="${spark_observe_report}" \
LOADER_SOURCE="${loader_handoff_report}" \
  bash "${ROOT_DIR}/scripts/complete-spark-hardware-proof.sh" \
  "${proof_finalize_dir}/qemu-stage-chain-proof.txt" >/dev/null
test -s "${proof_complete_dir}/latest-spark-hardware-proof-complete-status.txt"
grep -q '^overall_status=pass$' "${proof_complete_dir}/latest-spark-hardware-proof-complete-status.txt"
grep -q '^stage_chain_check_status=' "${proof_complete_dir}/latest-spark-hardware-proof-complete-status.txt"
printf 'spark_hardware_proof_complete=pass\n'
