# M0 Acceptance Report - 2026-03-30

This report records the first NovaOS scaffold pass against the original Spark-first handoff.

This note is historical. The current roadmap has since been widened into the portable-fabric direction, with Spark still kept as the first truth platform.

## Completed

- repository scaffold created under the initial Spark-first layout
- BootInfo v1 contract written in C at `abi/boot/nova_bootinfo_v1.h`
- `spark-observe` AA64 EFI scaffold builds for `aarch64-unknown-uefi`
- `novaaa64` stage0 EFI scaffold builds for `aarch64-unknown-uefi`
- `boot/stage1` scaffold builds for `aarch64-unknown-none-softfloat`
- `kernel/arch/arm64` scaffold builds for `aarch64-unknown-none-softfloat`
- local validation scripts check the workspace, UEFI target, kernel target, and ABI header
- continuous validation is installed as a user-systemd service

## Validated On This Host

- `cargo check --workspace`
- `cargo check -p spark-observe --target aarch64-unknown-uefi`
- `cargo check -p novaaa64 --target aarch64-unknown-uefi`
- `cargo check -p novaos-kernel --target aarch64-unknown-none-softfloat`
- `cargo check -p novaos-stage1`
- `./scripts/build-efi.sh`
- `./scripts/build-kernel.sh`
- `./scripts/run-qemu-spark-observe.sh`
- `./ci/validate-local.sh`

## Observed Result

QEMU Arm64 UEFI now auto-boots the removable `BOOTAA64.EFI` path and `spark-observe` prints a real NovaOS observatory report.

The current QEMU observatory output confirms:
- the AA64 EFI artifact boots under edk2
- firmware vendor and revision are readable
- the UEFI memory map can be queried
- ACPI and SMBIOS table presence can be detected
- Secure Boot state can be probed without guessing

## Current Gap

This report should not be read as the current M0 definition anymore.

The current M0 portability milestone is broader than this early scaffold pass, and M1 acceptance should still not be claimed from this note.

Remaining work:
- make `novaaa64` populate BootInfo from real UEFI facts instead of placeholders
- connect stage1 BootInfo parsing to the kernel scaffold
- validate the same observatory flow on actual Spark hardware and record the differences from QEMU
- begin adding portability hooks before the boot chain ossifies around Spark-specific assumptions

## Next Slice

- move BootInfo population logic into `novaaa64`
- carry the observatory facts into stage1 and then the kernel
- run `spark-observe.efi` on real Spark and capture a first platform dump
- draft the portability/fabric architecture note so later ports have an explicit target
