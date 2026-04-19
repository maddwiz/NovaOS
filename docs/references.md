# References

This document records the public sources that anchor the NovaOS Spark plan.

## Primary platform references

- NVIDIA DGX Spark product and platform documentation
- NVIDIA Spark UEFI and boot management documentation
- NVIDIA DGX OS / Spark software stack release notes
- NVIDIA open GPU kernel module documentation and source
- NVIDIA guidance on Spark Arm64 tuning and unified memory behavior

## Standards and implementation references

- UEFI specification
- PE/COFF specification
- ACPI specification
- Arm Architecture Reference Manual for A-profile
- Armv8-A / Armv9-A exception and memory model references
- EDK2 documentation for AA64 EFI applications

## Internal project references

- `docs/boot/spark-uefi-bringup.md`
- `docs/boot/bootinfo-v1.md`
- `docs/platform/spark-observatory.md`
- `docs/adr/0001-spark-scope.md`
- `docs/roadmap/m0-m2.md`

## Pinning rule

Any design note that depends on a vendor fact should record:
- source title
- source date or release version
- retrieval date
- the exact assumption it supports

That keeps later boot, kernel, and driver work traceable when the Spark software stack changes.

