# Spark UEFI Bring-up

This page describes the first operator workflow for NovaOS on DGX Spark.
Use `scripts/prepare-spark-hardware-bundle.sh` as the supported staging step before any real-hardware Spark proof, `scripts/install-spark-hardware-bundle.sh` as the privileged ESP install step, and `scripts/complete-spark-hardware-proof.sh` as the preferred post-boot return path once stage-chain evidence is available. That wrapper now validates the returned stage-chain markers against the current QEMU boot path. The lower-level collector scripts still exist when only one return artifact is available. The actual milestone stays open until an operator performs that real hardware boot/reboot flow and captures the resulting evidence.

## Goal

Boot a signed or test-signed EFI binary from Spark UEFI and capture enough information to validate the platform boundary before the kernel exists.

## First boot path

1. Run `bash ./scripts/prepare-spark-hardware-bundle.sh spark-observe` to stage an observatory bundle under `artifacts/hardware/`.
2. Preferred Linux operator path: run `sudo -E bash ./scripts/install-spark-hardware-bundle.sh spark-observe` to copy the staged observatory bundle onto the mounted ESP root, with the removable-media `\EFI\BOOT\BOOTAA64.EFI` path in place and the existing target files backed up first.
3. Optional one-time boot scheduling: rerun the install step with `USE_BOOTNEXT=1`, and optionally `REBOOT_AFTER=1`, if the operator wants the script to hand UEFI the next boot directly.
4. Manual fallback: copy the staged bundle contents onto a FAT-formatted USB stick or ESP so `\EFI\BOOT\BOOTAA64.EFI` exists at the removable-media boot path.
5. Use Spark UEFI one-time USB boot or the scheduled `BootNext` entry.
6. Boot the observatory EFI application first, not the kernel.
7. After returning to Linux, run `bash ./scripts/collect-spark-observe-report.sh` to ingest the report from the ESP and compare it with the QEMU baseline if the loader return is not back yet.
8. Once the loader return and stage-chain evidence are also available, prefer `bash ./scripts/complete-spark-hardware-proof.sh /path/to/stage-chain-evidence.txt` to finish the full return path in one command and validate the stage-chain markers.

## Loader proof path

1. Run `bash ./scripts/prepare-spark-hardware-bundle.sh novaaa64` to stage a loader bundle under `artifacts/hardware/`.
2. Preferred Linux operator path: run `sudo -E bash ./scripts/install-spark-hardware-bundle.sh novaaa64` to copy the staged loader bundle onto the mounted ESP root, preserving `\EFI\BOOT\BOOTAA64.EFI` plus the `\nova\` payload set from the staged bundle and backing up the prior target files first.
3. Optional one-time boot scheduling: rerun the install step with `USE_BOOTNEXT=1`, and optionally `REBOOT_AFTER=1`, if the operator wants the script to set `BootNext`.
4. Manual fallback: copy the staged bundle contents onto a FAT USB stick or ESP, keeping `\EFI\BOOT\BOOTAA64.EFI` beside `\nova\stage1.bin`, `\nova\kernel.bin`, and `\nova\init.capsule`.
5. Use Spark UEFI one-time boot or the scheduled `BootNext` entry to launch the prepared media.
6. After returning to Linux, prefer `bash ./scripts/collect-novaaa64-loader-report.sh` to ingest `\nova\loader\novaaa64-loader-report.txt` or `\EFI\BOOT\novaaa64-loader-report.txt` from the mounted ESP and validate it against the current QEMU baseline if the observatory return is not back yet.
7. If the run also exposes proof that `stage0 -> stage1 -> kernel` reached the current path through serial, display capture, or another log path, bring that evidence back too.
8. Once the observatory return and stage-chain evidence are also available, prefer `bash ./scripts/complete-spark-hardware-proof.sh /path/to/stage-chain-evidence.txt` to finish the full return path in one command and validate the stage-chain markers.

## What the observatory must capture

- firmware vendor and revision
- UEFI memory map
- current boot source
- Secure Boot state
- loaded image path
- GOP mode and framebuffer geometry
- config table GUID inventory
- ACPI RSDP presence
- device-tree pointer presence
- SMBIOS presence
- obvious block and boot device handles

## Bring-up rules

- Keep Secure Boot disabled for unstable early loader work.
- Do not assume ACPI or device tree until the observatory proves it.
- Do not assume the integrated GPU is exposed as a PCIe device.
- Use framebuffer output and persistent logs before any native display stack exists.

## Expected outputs

- an on-screen summary
- a structured boot report
- for loader runs, a structured `novaaa64` handoff report on EFI media
- a short human-readable note describing what was discovered

## Compare With QEMU

After copying a real Spark structured report into the workspace, compare it against the current QEMU baseline with:

```bash
cd /home/nova/NovaOS
./scripts/compare-spark-observe-reports.sh /path/to/real-spark-report.txt
```

That writes a comparison artifact into `artifacts/reports/` and updates `latest-spark-observe-compare.md`.
If the report is still on the mounted ESP, prefer `bash ./scripts/collect-spark-observe-report.sh`, which copies it into `artifacts/reports/` first and then runs the comparison automatically.

## Acceptance for M0

Spark UEFI can boot the observatory app and produce a stable report that can be compared across firmware revisions.
The prep/install/collect scripts define the supported operator workflow for that proof. The real-hardware acceptance item remains unchecked until those operator/root/reboot steps finish on a Spark machine.

## Complete Returned Proof

After the observatory report, loader handoff report, and at least one `stage0 -> stage1 -> kernel` evidence file are back in the workspace, prefer the one-command return path:

```bash
cd /home/nova/NovaOS
bash ./scripts/complete-spark-hardware-proof.sh /path/to/stage-chain-evidence.txt
```

That ingests both returned reports from the mounted ESP, validates the stage0/stage1/kernel markers against the current QEMU boot path, runs the existing report checks, and writes `artifacts/reports/latest-spark-hardware-proof.md` plus matching status files so another Codex session can see whether the current Spark hardware proof set is actually complete.
