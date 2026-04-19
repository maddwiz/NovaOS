# Boot Common

Shared boot-lane code belongs here as NovaOS grows beyond the first Spark lane.

Rules:

- put UEFI-independent boot contracts here
- do not put Spark-specific constants here
- keep Arm64- and x86_64-specific entry details out of this directory
