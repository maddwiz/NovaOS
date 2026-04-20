#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${ROOT_DIR:-/home/nova/NovaOS}"
export ROOT_DIR

EFI_BINARY=novaaa64 \
EXPECT_TEXTS="NovaOS stage0 loader;NovaOS stage1 entered;NovaOS stage1 bootinfo_v2 sidecar;NovaOS kernel entered;NovaOS kernel bring-up;init capsule summary observed;bootstrap task current initd;bootstrap task launch plan from bootinfo_v2;bootstrap capability probe passed;bootstrap endpoint probe passed;bootstrap shared memory probe passed;bootstrap lower-el svc dry-run passed;bootstrap el0 mapping ready;bootstrap task transfer initd;bootstrap task boundary same-el;bootstrap task target boundary drop-to-el0;NovaOS initd payload entered;NovaOS initd bootstrap context ready;bootstrap kernel call from initd;NovaOS initd bootstrap kernel call passed;NovaOS initd bootstrap svc passed" \
STAGE_NOVA_PAYLOADS=1 \
PAYLOAD_FEATURES=qemu_virt_trace \
INITD_FEATURES=qemu_virt_trace,bootstrap_svc_probe \
NOVAAA64_FEATURES=qemu_virt_trace \
TIMEOUT_SECONDS="${TIMEOUT_SECONDS:-8}" \
  "${ROOT_DIR}/scripts/run-qemu-spark-observe.sh"
