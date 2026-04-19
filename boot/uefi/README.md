# Boot UEFI

Shared UEFI helper code for Arm64 and future x86_64 boot lanes belongs here.

Rules:

- keep reusable UEFI helpers here
- keep platform-specific observatory or loader quirks in the relevant platform lane
- do not hide Spark-only behavior behind generic names
