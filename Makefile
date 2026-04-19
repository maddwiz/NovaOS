ROOT_DIR ?= /home/nova/NovaOS

.PHONY: check env report loop build-efi build-kernel qemu-observe qemu-stage0 install-service service-status

check:
	ROOT_DIR=$(ROOT_DIR) ./ci/validate-local.sh

env:
	ROOT_DIR=$(ROOT_DIR) ./scripts/novaos-env-check.sh

report:
	ROOT_DIR=$(ROOT_DIR) ./scripts/novaos-report.sh

loop:
	ROOT_DIR=$(ROOT_DIR) RUN_ONCE=1 ./scripts/novaos-ci-loop.sh

build-efi:
	ROOT_DIR=$(ROOT_DIR) ./scripts/build-efi.sh

build-kernel:
	ROOT_DIR=$(ROOT_DIR) ./scripts/build-kernel.sh

qemu-observe:
	ROOT_DIR=$(ROOT_DIR) ./scripts/run-qemu-spark-observe.sh

qemu-stage0:
	ROOT_DIR=$(ROOT_DIR) ./scripts/run-qemu-novaaa64.sh

install-service:
	ROOT_DIR=$(ROOT_DIR) ./scripts/install-novaos-user-service.sh

service-status:
	systemctl --user --no-pager --full status novaos-validation.service
