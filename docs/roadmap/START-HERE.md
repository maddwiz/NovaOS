# NovaOS Start Here

Use this file when resuming NovaOS from another Codex session or another machine.

NovaOS is now a portable fabric OS with Spark as the first truth platform. Keep the checklist and live status files as the source of truth for continuation.

## Read Order

1. [live-status.md](live-status.md)
2. [master-roadmap-checklist.md](master-roadmap-checklist.md)
3. [portable-fabric.md](../architecture/portable-fabric.md)
4. [m1-progress-2026-03-30.md](m1-progress-2026-03-30.md)
5. `artifacts/reports/latest-report.md` when generated locally

## Minimum Resume Steps

1. Confirm the automation loop is still active.
2. Read the latest green or failing report.
3. Read the master checklist and live status before changing code.
4. Run `./ci/validate-local.sh` before changing the boot path or contract docs.
5. For manual real Spark proof, use `scripts/prepare-spark-hardware-bundle.sh` to stage the bundle, `scripts/install-spark-hardware-bundle.sh` for the privileged ESP install path, and `scripts/complete-spark-hardware-proof.sh` once the machine returns and stage-chain evidence is available. That wrapper now validates the returned stage-chain markers against the current QEMU boot path. The lower-level collect/finalize scripts remain available for partial returns.
6. Continue from the first unchecked item in the master roadmap checklist.

## Automation

- service: `novaos-validation.service`
- status command: `systemctl --user status novaos-validation.service --no-pager`
- log command: `journalctl --user -u novaos-validation.service -n 60 --no-pager`
- reports: local runtime outputs under `artifacts/reports/`; run `make report` if they are absent in a fresh clone
