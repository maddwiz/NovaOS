# Spark Observatory

The observatory is the first executable milestone for NovaOS.

It exists to answer one question: what does Spark UEFI actually hand to the OS?

## Responsibilities

- identify the firmware vendor and revision
- dump the memory map
- report the active boot source
- report Secure Boot state
- capture framebuffer geometry
- record available config tables
- detect ACPI RSDP, DTB, and SMBIOS presence
- enumerate obvious boot and storage handles

## Output forms

- on-screen summary
- structured file report written to `\nova\observatory\spark-observe-report.txt`
- compact binary blob if needed for later tooling
- comparison report against the QEMU baseline using `scripts/compare-spark-observe-reports.sh`

## Manual Hardware Flow

Use `bash ./scripts/prepare-spark-hardware-bundle.sh spark-observe` to stage the observatory bundle under `artifacts/hardware/`.
Use `sudo -E bash ./scripts/install-spark-hardware-bundle.sh spark-observe` as the supported privileged install step for the mounted ESP. That copies the staged bundle onto the ESP root, backs up the previous `\EFI\BOOT\BOOTAA64.EFI` target first, can optionally schedule `BootNext`, and preserves the primary `\nova\observatory\` report path from the bundle layout.
After the machine returns to Linux, use `bash ./scripts/collect-spark-observe-report.sh` to pull `spark-observe-report.txt` back into `artifacts/reports/` and run the comparison against the QEMU baseline.
The milestone still stays open until that real hardware boot happens and the report comes back into the workspace.

## Why this matters

NovaOS cannot safely hard-code platform assumptions before the observatory has confirmed them on real Spark hardware.

The observatory is also the first debugging tool for loader work, because it exposes firmware behavior before the kernel exists.

## Success criteria

- runs as a standalone AA64 EFI application
- produces stable output across repeated boots
- gives enough information to fill BootInfo v2 display, storage, and accelerator-seed facts without guessing
- records BootInfo v2-shaped display and storage seed paths plus a transitional Spark UMA accelerator-seed draft
- can be compared with the persisted QEMU baseline without manual key-by-key review
